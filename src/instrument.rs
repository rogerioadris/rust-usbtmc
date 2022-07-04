use crate::error::UsbtmcError;
use crate::helper::UsbtmcResult;
use byteorder::{ByteOrder, LittleEndian};
use core::time::Duration;
use rusb::Context;
use rusb::Device;
use rusb::DeviceDescriptor;
use rusb::DeviceHandle;
use rusb::Direction;
use rusb::TransferType;
use rusb::UsbContext;
use std::str;

const USBTMC_MSGID_DEV_DEP_MSG_OUT: u8 = 1;
const USBTMC_MSGID_DEV_DEP_MSG_IN: u8 = 2;

#[derive(Debug)]
pub struct Endpoint {
    pub config: u8,
    pub iface: u8,
    pub setting: u8,
    pub address: u8,
}

pub struct Instrument {
    pub vid: u16,
    pub pid: u16,
    pub bus: Option<u8>,
    pub address: Option<u8>,
    last_btag: u8,
    max_transfer_size: u32,
    timeout: Duration,
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
        }
    }

    ///
    ///
    ///
    pub fn new_filtered(vid: u16, pid: u16, bus: u8, address: u8) -> Instrument {
        Instrument {
            vid,
            pid,
            bus: Some(bus),
            address: Some(address),
            last_btag: 0,
            max_transfer_size: 1024 * 1024,
            timeout: Duration::from_secs(1),
        }
    }

    /// Return Instrument information
    pub fn info(&self) -> UsbtmcResult<String> {
        Ok(String::new())
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
        let (mut device, device_desc, mut handle) = self.open_device()?;

        match self.find_endpoint(&mut device, &device_desc, TransferType::Bulk, Direction::In) {
            Some(endpoint) => {
                let has_kernel_driver = match handle.kernel_driver_active(endpoint.iface) {
                    Ok(true) => {
                        handle.detach_kernel_driver(endpoint.iface).ok();
                        true
                    }
                    _ => false,
                };

                let buf = &mut [0u8; 1024];
                handle.read_bulk(endpoint.address, buf, self.timeout)?;

                let line_size = buf
                    .iter()
                    .take_while(|c| **c != b'\n' && **c != b'\r')
                    .count();

                let result = str::from_utf8(&buf[12..line_size]).unwrap().to_string();

                if has_kernel_driver {
                    handle.attach_kernel_driver(endpoint.iface).ok();
                }

                Ok(result)
            }

            None => return Err(UsbtmcError::BulkOut),
        }
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
        vec![msgid, btag, !btag & 0xFF, 0x00]
    }

    fn pack_dev_dep_msg_out_header(&mut self, transfer_size: usize, eom: bool) -> Vec<u8> {
        let mut hdr = self.pack_bulk_out_header(USBTMC_MSGID_DEV_DEP_MSG_OUT);

        hdr.append(&mut self.little_write_u32(transfer_size as u32, 4));
        hdr.push(if eom { 0x01 } else { 0x00 });
        hdr.append(&mut vec![0x00; 3]);

        hdr
    }

    fn pack_dev_dep_msg_in_header(&mut self, transfer_size: usize, term_char: u8) -> Vec<u8> {
        let mut hdr = self.pack_bulk_out_header(USBTMC_MSGID_DEV_DEP_MSG_IN);

        hdr.append(&mut self.little_write_u32(transfer_size as u32, 4));
        hdr.push(if term_char == 0 { 0x00 } else { 0x02 });
        hdr.push(term_char);
        hdr.append(&mut vec![0x00; 2]);

        hdr
    }

    fn little_write_u32(&self, size: u32, len: u8) -> Vec<u8> {
        let mut buf = vec![0; len as usize];
        LittleEndian::write_u32(&mut buf, size);

        buf
    }

    fn open_device(
        &self,
    ) -> UsbtmcResult<(Device<Context>, DeviceDescriptor, DeviceHandle<Context>)> {
        let context = Context::new().unwrap();
        let devices = match context.devices() {
            Ok(list) => list,
            Err(_) => return Err(UsbtmcError::Exception),
        };

        for device in devices.iter() {
            let device_desc = match device.device_descriptor() {
                Ok(descriptor) => descriptor,
                Err(_) => continue,
            };

            if device_desc.vendor_id() == self.vid && device_desc.product_id() == self.pid {
                if self.bus.is_some() && self.address.is_some() {
                    if device.bus_number() == self.bus.unwrap() && device.address() == self.address.unwrap() {
                        match device.open() {
                            Ok(handle) => return Ok((device, device_desc, handle)),
                            Err(_) => continue,
                        }
                    }
                } else {
                    match device.open() {
                        Ok(handle) => return Ok((device, device_desc, handle)),
                        Err(_) => continue,
                    }
                }
            }
        }
        return Err(UsbtmcError::Exception);
    }

    fn find_endpoint(
        &mut self,
        device: &mut Device<Context>,
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
        let offset: usize = 0;
        let mut eom: bool = false;
        let mut num: usize = data.len();

        let (mut device, device_desc, mut handle) = self.open_device()?;

        match self.find_endpoint(
            &mut device,
            &device_desc,
            TransferType::Bulk,
            Direction::Out,
        ) {
            Some(endpoint) => {
                let has_kernel_driver = match handle.kernel_driver_active(endpoint.iface) {
                    Ok(true) => {
                        handle.detach_kernel_driver(endpoint.iface).ok();
                        true
                    }
                    _ => false,
                };

                while num > 0 {
                    if num <= self.max_transfer_size as usize {
                        eom = true;
                    }

                    let block = &data[offset..(num - offset)];
                    let size: usize = block.len();

                    let mut req = self.pack_dev_dep_msg_out_header(size, eom);
                    let mut b: Vec<u8> = block.iter().cloned().collect();
                    req.append(&mut b);
                    req.append(&mut vec![0x00; (4 - (size % 4)) % 4]);

                    handle.write_bulk(endpoint.address, &req, self.timeout)?;

                    num = num - size;
                }

                if command {
                    let send = self.pack_dev_dep_msg_in_header(self.max_transfer_size as usize, 0);
                    handle.write_bulk(endpoint.address, &send, self.timeout)?;
                }

                if has_kernel_driver {
                    handle.attach_kernel_driver(endpoint.iface).ok();
                }
            }

            None => return Err(UsbtmcError::BulkOut),
        }

        Ok(())
    }
}
