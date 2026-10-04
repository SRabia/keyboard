pub mod terminal {
    pub const MAX_BUF_OUT_LINE: usize = 1024;
    pub const MAX_BUF_IN_LINE: usize = 258;
    pub const DEFAULT_PROMPT: &str = "> ";
    pub const PROMPT_LEN: usize = 128;
}

pub const LOGGER_BUFFER_MAX: usize = 1024;

pub mod usb {
    pub const VID: u16 = 0xc0de;
    pub const PID: u16 = 0xcafe;
    pub const MAN: &str = "awesome";
    pub const PRODUCT: &str = "my-keyboard";
    pub const SERIAL_NB: &str = "12345678";
    pub const MAX_POWER: u16 = 100;
    pub const MAX_PACKET_SIZE_0: usize = 64;
}
