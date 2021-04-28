use crate::helper::UsbtmcResult;

pub struct Instrument {
    pub vid: u16,
    pub pid: u16,
}

impl Instrument {
    ///
    ///
    ///
    pub fn new(vid: u16, pid: u16) -> Instrument {
        Instrument { vid, pid }
    }

    ///
    ///
    ///
    pub fn info(&self) -> UsbtmcResult<String> {
        Ok(String::new())
    }

    ///
    ///
    ///
    pub fn write(&mut self, message: &str) -> UsbtmcResult<()> {
        self.write_raw(message.as_bytes())
    }

    ///
    ///
    ///
    pub fn write_raw(&mut self, _data: &[u8]) -> UsbtmcResult<()> {
        Ok(())
    }

    ///
    ///
    ///
    pub fn read(&mut self) -> UsbtmcResult<String> {
        self.read_raw()
    }

    ///
    ///
    ///
    pub fn read_raw(&mut self) -> UsbtmcResult<String> {
        Ok(String::new())
    }

    ///
    ///
    ///
    pub fn ask(&mut self, data: &str) -> UsbtmcResult<String> {
        self.ask_raw(data.as_bytes())
    }

    ///
    ///
    ///
    pub fn ask_raw(&mut self, _data: &[u8]) -> UsbtmcResult<String> {
        Ok(String::new())
    }
}
