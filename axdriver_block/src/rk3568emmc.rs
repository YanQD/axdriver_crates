//! SD card driver for raspi4

extern crate alloc;
use crate::BlockDriverOps;
use axdriver_base::{BaseDriverOps, DevError, DevResult, DeviceType};
use dma_api::{DVec, Direction};
use sdmmc::emmc::EMmcHost;
use sdmmc::err::SdError;
use sdmmc::BLOCK_SIZE;

//pub const EMMC_BASE: usize = IO_BASE + 0x300000;
pub const EMMC_BASE: usize = 0xFE350000;

/// RK3568 SDHCI driver (Raspberry Pi SD card).
pub struct EmmcDriver(EMmcHost);

impl EmmcDriver {
    /// Initialize the SDHCI driver, returns `Ok` if successful.
    pub fn try_new() -> DevResult<EmmcDriver> {
        let mut ctrl = EMmcHost::new(0xFE350000);
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
        let mut temp_buffer = match DVec::zeros(BLOCK_SIZE, 0x1000, Direction::FromDevice) {
            Some(buffer) => buffer,
            None => return Err(DevError::NoMemory),
        };

        self.0.read_blocks(block_id as u32, 1, &mut temp_buffer)
            .map_err(deal_emmc_err)?;

        let copy_size = buf.len().min(BLOCK_SIZE);

        temp_buffer.to_vec()
            .iter()
            .take(copy_size)
            .enumerate()
            .for_each(|(i, &byte)| buf[i] = byte);

        Ok(())
    }

    fn write_block(&mut self, block_id: u64, buf: &[u8]) -> DevResult {
        let mut temp_buffer = match DVec::zeros(BLOCK_SIZE, 0x1000, Direction::ToDevice) {
            Some(buffer) => buffer,
            None => return Err(DevError::NoMemory),
        };

        let copy_size = buf.len().min(BLOCK_SIZE);
        for i in 0..copy_size {
            temp_buffer.set(i, buf[i]);
        }

        self.0.write_blocks(block_id as u32, 1, &mut temp_buffer)
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
