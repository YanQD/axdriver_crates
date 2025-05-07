//! SD card driver for raspi4

extern crate alloc;
use crate::BlockDriverOps;
use axdriver_base::{BaseDriverOps, DevError, DevResult, DeviceType};
use dma_api::{DVec, Direction};
use sdmmc::emmc::EMmcHost;
use sdmmc::err::SdError;
use sdmmc::BLOCK_SIZE;
use sdmmc::emmc::clock::init_clk;

//pub const EMMC_BASE: usize = IO_BASE + 0x300000;
pub const EMMC_BASE: usize = 0xFE310000;

/// RK3568 SDHCI driver (Raspberry Pi SD card).
pub struct EmmcDriver(EMmcHost);

impl EmmcDriver {
    /// Initialize the SDHCI driver, returns `Ok` if successful.
    pub fn try_new(emmc_base: usize) -> DevResult<EmmcDriver> {
        let mut ctrl = EMmcHost::new(emmc_base);
        let _ = init_clk(0xffff_0000_fdd2_0000);
        if ctrl.init().is_ok() {
            log::info!("RK3568 emmc: successfully initialized");
            Ok(EmmcDriver(ctrl))
        } else {
            log::warn!("RK3568 emmc: init failed");
            Err(DevError::Io)
        }
    }
}

fn deal_emmc_err(err: SdError) -> DevError {
    match err {
        SdError::Timeout | SdError::DataTimeout => DevError::Again,
        SdError::Crc | SdError::EndBit | SdError::Index | 
        SdError::DataCrc | SdError::DataEndBit | 
        SdError::DataError => DevError::Io,
        SdError::BusPower | SdError::CurrentLimit => DevError::Io,
        SdError::Acmd12Error | SdError::AdmaError => DevError::Io,
        SdError::InvalidResponse | SdError::InvalidResponseType => DevError::BadState,
        SdError::NoCard => DevError::ResourceBusy,
        SdError::UnsupportedCard => DevError::Unsupported,
        SdError::IoError | SdError::TransferError => DevError::Io,
        SdError::CommandError => DevError::Io,
        SdError::TuningFailed | SdError::VoltageSwitchFailed => DevError::BadState,
        SdError::BadMessage => DevError::InvalidParam,
        SdError::InvalidArgument => DevError::InvalidParam,
        SdError::BufferOverflow => DevError::NoMemory,
        SdError::MemoryError => DevError::NoMemory,
        SdError::BusWidth => DevError::BadState,
        SdError::CardError(_, _) => DevError::Io,
    }
}

impl BaseDriverOps for EmmcDriver {
    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn device_name(&self) -> &str {
        "rk3568_emmc"
    }
}

impl BlockDriverOps for EmmcDriver {
    fn read_block(&mut self, block_id: u64, buf: &mut [u8]) -> DevResult {
        if buf.len() < BLOCK_SIZE {
            return Err(DevError::InvalidParam);
        }
        
        // 检查对齐
        let (prefix, _, suffix) = unsafe { buf.align_to_mut::<u32>() };
        if !prefix.is_empty() || !suffix.is_empty() {
            return Err(DevError::InvalidParam);
        }
        
        // 直接使用原始的字节缓冲区
        self.0
            .read_blocks(block_id as u32, 1, buf)
            .map_err(deal_emmc_err)
    }

    fn write_block(&mut self, block_id: u64, buf: &[u8]) -> DevResult {
        if buf.len() < BLOCK_SIZE {
            return Err(DevError::Io);
        }
        let (prefix, _, suffix) = unsafe { buf.align_to::<u32>() };
        if !prefix.is_empty() || !suffix.is_empty() {
            return Err(DevError::InvalidParam);
        }
        self.0
            .write_blocks(block_id as u32, 1, buf)
            .map_err(deal_emmc_err)
    }

    fn flush(&mut self) -> DevResult {
        Ok(())
    }

    #[inline]
    fn num_blocks(&self) -> u64 {
        self.0.get_block_num()
    }

    #[inline]
    fn block_size(&self) -> usize {
        self.0.get_block_size()
    }
}
