use crate::error::UsbtmcError;
use crate::helper::UsbtmcResult;
use core::time::Duration;
use rusb::Context;
use rusb::Device;
use rusb::DeviceDescriptor;
use rusb::DeviceHandle;
use rusb::Direction;
use rusb::TransferType;
use rusb::UsbContext;

const USBTMC_MSGID_DEV_DEP_MSG_OUT: u8 = 1;
const USBTMC_MSGID_DEV_DEP_MSG_IN: u8 = 2;
const NUMBER_OF_HEADER_BYTES: usize = 12;
const READ_BUFFER_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy)]
pub struct Endpoint {
    pub config: u8,
    pub iface: u8,
    pub setting: u8,
    pub address: u8,
}

struct Connection {
    handle: DeviceHandle<Context>,
    out_endpoint: Endpoint,
    in_endpoint: Endpoint,
    detached_ifaces: Vec<u8>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        for iface in &self.detached_ifaces {
            self.handle.attach_kernel_driver(*iface).ok();
        }
    }
}

pub struct Instrument {
    pub vid: u16,
    pub pid: u16,
    pub bus: Option<u8>,
    pub address: Option<u8>,
    last_btag: u8,
    max_transfer_size: u32,
    timeout: Duration,
    connection: Option<Connection>,
}

impl Instrument {
    /// Create a new Instrument with speciifed VID and PID.
    pub fn new(vid: u16, pid: u16) -> Instrument {
        Instrument {
            vid,
            pid,
            bus: None,
            address: None,
            last_btag: 0,
            max_transfer_size: 1024 * 1024,
            timeout: Duration::from_secs(1),
            connection: None,
        }
    }

    /// Create a new Instrument with speciifed VID, PID, Bus and Address.
    pub fn new_filtered(vid: u16, pid: u16, bus: u8, address: u8) -> Instrument {
        Instrument {
            vid,
            pid,
            bus: Some(bus),
            address: Some(address),
            last_btag: 0,
            max_transfer_size: 1024 * 1024,
            timeout: Duration::from_secs(1),
            connection: None,
        }
    }

    /// Set the timeout used for USB bulk transfers.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Set the maximum transfer size (bytes) used when writing/reading messages.
    pub fn set_max_transfer_size(&mut self, max_transfer_size: u32) {
        self.max_transfer_size = max_transfer_size;
    }

    /// Return Instrument identification string (`*IDN?`).
    pub fn info(&mut self) -> UsbtmcResult<String> {
        self.ask("*IDN?")
    }

    /// Write a message to the Instrument
    pub fn write(&mut self, message: &str) -> UsbtmcResult<()> {
        self.write_raw(message.as_bytes())
    }

    /// Write a byte array to the Instrument
    pub fn write_raw(&mut self, data: &[u8]) -> UsbtmcResult<()> {
        self.write_data(data, false)
    }

    /// Read string from the instrument
    pub fn read(&mut self) -> UsbtmcResult<String> {
        self.read_raw()
    }

    /// Read string from the instrument
    pub fn read_raw(&mut self) -> UsbtmcResult<String> {
        self.ensure_connection()?;
        let timeout = self.timeout;

        let mut message = Vec::new();
        let mut buf = [0u8; READ_BUFFER_SIZE];

        loop {
            let connection = self.connection.as_ref().unwrap();
            let bytes_read =
                connection
                    .handle
                    .read_bulk(connection.in_endpoint.address, &mut buf, timeout)?;

            if bytes_read < NUMBER_OF_HEADER_BYTES {
                return Err(UsbtmcError::BulkIn);
            }

            let transfer_size = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
            let eom = buf[8] & 0x01 != 0;
            let available = bytes_read - NUMBER_OF_HEADER_BYTES;
            let payload_len = transfer_size.min(available);

            message.extend_from_slice(
                &buf[NUMBER_OF_HEADER_BYTES..NUMBER_OF_HEADER_BYTES + payload_len],
            );

            if eom {
                break;
            }
        }

        while matches!(message.last(), Some(b'\n') | Some(b'\r')) {
            message.pop();
        }

        String::from_utf8(message).map_err(|e| UsbtmcError::Utf8(e.utf8_error()))
    }

    /// Send a message and wait for response
    pub fn ask(&mut self, data: &str) -> UsbtmcResult<String> {
        self.ask_raw(data.as_bytes())
    }

    /// Send data and wait for response
    pub fn ask_raw(&mut self, data: &[u8]) -> UsbtmcResult<String> {
        self.write_data(data, true)?;
        self.read_raw()
    }

    fn pack_bulk_out_header(&mut self, msgid: u8) -> Vec<u8> {
        let btag: u8 = (self.last_btag % 255) + 1;
        self.last_btag = btag;

        // BBBx
        vec![msgid, btag, !btag, 0x00]
    }

    fn pack_dev_dep_msg_out_header(&mut self, transfer_size: usize, eom: bool) -> Vec<u8> {
        let mut hdr = self.pack_bulk_out_header(USBTMC_MSGID_DEV_DEP_MSG_OUT);

        hdr.extend_from_slice(&(transfer_size as u32).to_le_bytes());
        hdr.push(if eom { 0x01 } else { 0x00 });
        hdr.extend_from_slice(&[0x00; 3]);

        hdr
    }

    fn pack_dev_dep_msg_in_header(&mut self, transfer_size: usize, term_char: u8) -> Vec<u8> {
        let mut hdr = self.pack_bulk_out_header(USBTMC_MSGID_DEV_DEP_MSG_IN);

        hdr.extend_from_slice(&(transfer_size as u32).to_le_bytes());
        hdr.push(if term_char == 0 { 0x00 } else { 0x02 });
        hdr.push(term_char);
        hdr.extend_from_slice(&[0x00; 2]);

        hdr
    }

    /// Open (or reuse) the USB connection: find the device, claim its bulk
    /// endpoints and cache the handle so subsequent calls skip re-enumeration.
    fn ensure_connection(&mut self) -> UsbtmcResult<()> {
        if self.connection.is_some() {
            return Ok(());
        }

        let context = Context::new()?;
        let devices = context.devices()?;

        for device in devices.iter() {
            let device_desc = match device.device_descriptor() {
                Ok(descriptor) => descriptor,
                Err(_) => continue,
            };

            if device_desc.vendor_id() != self.vid || device_desc.product_id() != self.pid {
                continue;
            }

            if let (Some(bus), Some(address)) = (self.bus, self.address) {
                if device.bus_number() != bus || device.address() != address {
                    continue;
                }
            }

            let out_endpoint =
                match self.find_endpoint(&device, &device_desc, TransferType::Bulk, Direction::Out)
                {
                    Some(endpoint) => endpoint,
                    None => continue,
                };

            let in_endpoint = match self.find_endpoint(
                &device,
                &device_desc,
                TransferType::Bulk,
                Direction::In,
            ) {
                Some(endpoint) => endpoint,
                None => continue,
            };

            let handle = match device.open() {
                Ok(handle) => handle,
                Err(_) => continue,
            };

            let mut detached_ifaces = Vec::new();
            for iface in [out_endpoint.iface, in_endpoint.iface] {
                if detached_ifaces.contains(&iface) {
                    continue;
                }
                if handle.kernel_driver_active(iface).unwrap_or(false) {
                    handle.detach_kernel_driver(iface)?;
                    detached_ifaces.push(iface);
                }
                handle.claim_interface(iface)?;
            }

            self.connection = Some(Connection {
                handle,
                out_endpoint,
                in_endpoint,
                detached_ifaces,
            });

            return Ok(());
        }

        Err(UsbtmcError::Exception)
    }

    fn find_endpoint(
        &self,
        device: &Device<Context>,
        device_desc: &DeviceDescriptor,
        transfer_type: TransferType,
        direction: Direction,
    ) -> Option<Endpoint> {
        for index in 0..device_desc.num_configurations() {
            let config_desc = match device.config_descriptor(index) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for interface in config_desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    for endpoint_desc in interface_desc.endpoint_descriptors() {
                        if endpoint_desc.transfer_type() == transfer_type
                            && endpoint_desc.direction() == direction
                        {
                            return Some(Endpoint {
                                config: config_desc.number(),
                                iface: interface_desc.interface_number(),
                                setting: interface_desc.setting_number(),
                                address: endpoint_desc.address(),
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn write_data(&mut self, data: &[u8], command: bool) -> UsbtmcResult<()> {
        self.ensure_connection()?;

        let timeout = self.timeout;
        let max_transfer_size = self.max_transfer_size as usize;
        let total = data.len();
        let mut offset: usize = 0;

        loop {
            let remaining = total - offset;
            let chunk_size = remaining.min(max_transfer_size);
            let eom = offset + chunk_size == total;

            let mut req = self.pack_dev_dep_msg_out_header(chunk_size, eom);
            req.extend_from_slice(&data[offset..offset + chunk_size]);
            let padding = (4 - (chunk_size % 4)) % 4;
            req.resize(req.len() + padding, 0x00);

            let connection = self.connection.as_ref().unwrap();
            connection
                .handle
                .write_bulk(connection.out_endpoint.address, &req, timeout)?;

            offset += chunk_size;
            if offset >= total {
                break;
            }
        }

        if command {
            let send = self.pack_dev_dep_msg_in_header(self.max_transfer_size as usize, 0);
            let connection = self.connection.as_ref().unwrap();
            connection
                .handle
                .write_bulk(connection.out_endpoint.address, &send, timeout)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_out_header_increments_btag() {
        let mut instr = Instrument::new(0x0000, 0x0000);

        let hdr = instr.pack_bulk_out_header(USBTMC_MSGID_DEV_DEP_MSG_OUT);
        assert_eq!(hdr, vec![USBTMC_MSGID_DEV_DEP_MSG_OUT, 1, !1u8, 0x00]);

        let hdr2 = instr.pack_bulk_out_header(USBTMC_MSGID_DEV_DEP_MSG_OUT);
        assert_eq!(hdr2[1], 2);
    }

    #[test]
    fn btag_wraps_after_255() {
        let mut instr = Instrument::new(0x0000, 0x0000);
        instr.last_btag = 254;

        let hdr = instr.pack_bulk_out_header(USBTMC_MSGID_DEV_DEP_MSG_OUT);
        assert_eq!(hdr[1], 255);

        let hdr2 = instr.pack_bulk_out_header(USBTMC_MSGID_DEV_DEP_MSG_OUT);
        assert_eq!(hdr2[1], 1);
    }

    #[test]
    fn dev_dep_msg_out_header_encodes_size_and_eom() {
        let mut instr = Instrument::new(0x0000, 0x0000);

        let hdr = instr.pack_dev_dep_msg_out_header(300, true);
        assert_eq!(hdr.len(), NUMBER_OF_HEADER_BYTES);
        assert_eq!(&hdr[4..8], &300u32.to_le_bytes());
        assert_eq!(hdr[8], 0x01);
        assert_eq!(&hdr[9..12], &[0x00, 0x00, 0x00]);

        let hdr_no_eom = instr.pack_dev_dep_msg_out_header(1024, false);
        assert_eq!(hdr_no_eom[8], 0x00);
    }

    #[test]
    fn dev_dep_msg_in_header_encodes_term_char() {
        let mut instr = Instrument::new(0x0000, 0x0000);

        let hdr = instr.pack_dev_dep_msg_in_header(1024, 0);
        assert_eq!(hdr.len(), NUMBER_OF_HEADER_BYTES);
        assert_eq!(&hdr[4..8], &1024u32.to_le_bytes());
        assert_eq!(hdr[8], 0x00);
        assert_eq!(hdr[9], 0x00);

        let hdr_term = instr.pack_dev_dep_msg_in_header(1024, b'\n');
        assert_eq!(hdr_term[8], 0x02);
        assert_eq!(hdr_term[9], b'\n');
    }
}
