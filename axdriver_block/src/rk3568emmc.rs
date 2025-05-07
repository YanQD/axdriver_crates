//! eMMC driver for ROC-RK3568-PC development board

extern crate alloc;

use crate::BlockDriverOps;
use axdriver_base::{BaseDriverOps, DevError, DevResult, DeviceType};
use sdmmc::emmc::EMmcHost;
use sdmmc::err::SdError;
use sdmmc::BLOCK_SIZE;
use sdmmc::emmc::clock::init_clk;

// Base address for the RK3568 eMMC controller
pub const EMMC_BASE: usize = 0xFE310000;
pub const CRU_BASE: usize = 0xFFFF_0000_FDD2_0000;

/// Driver for the RK3568 eMMC controller.
pub struct EmmcDriver(EMmcHost);

impl EmmcDriver {
    /// Attempt to initialize the eMMC driver.
    ///
    /// # Arguments
    ///
    /// * `emmc_base` - Physical base address of the eMMC controller.
    ///
    /// # Returns
    ///
    /// * `Ok(EmmcDriver)` on success.
    /// * `Err(DevError::Io)` if initialization fails.
    pub fn try_new(emmc_base: usize) -> DevResult<EmmcDriver> {
        let mut ctrl = EMmcHost::new(emmc_base);
        let _ = init_clk(CRU_BASE);
        if ctrl.init().is_ok() {
            log::info!("RK3568 eMMC: successfully initialized");
            Ok(EmmcDriver(ctrl))
        } else {
            log::warn!("RK3568 eMMC: init failed");
            Err(DevError::Io)
        }
    }
}

/// Maps `SdError` values from the lower-level driver to generic `DevError`s.
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
        SdError::BadMessage | SdError::InvalidArgument => DevError::InvalidParam,
        SdError::BufferOverflow | SdError::MemoryError => DevError::NoMemory,
        SdError::BusWidth => DevError::BadState,
        SdError::CardError(_, _) => DevError::Io,
    }
}

impl BaseDriverOps for EmmcDriver {
    /// Returns the device type as a block device.
    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    /// Returns the name of the device for identification.
    fn device_name(&self) -> &str {
        "rk3568_emmc"
    }
}

impl BlockDriverOps for EmmcDriver {
    /// Reads a single block from the eMMC device into the provided buffer.
    fn read_block(&mut self, block_id: u64, buf: &mut [u8]) -> DevResult {
        if buf.len() < BLOCK_SIZE {
            return Err(DevError::InvalidParam);
        }

        let (prefix, _, suffix) = unsafe { buf.align_to_mut::<u32>() };
        if !prefix.is_empty() || !suffix.is_empty() {
            return Err(DevError::InvalidParam);
        }

        self.0
            .read_blocks(block_id as u32, 1, buf)
            .map_err(deal_emmc_err)
    }

    /// Writes a single block to the eMMC device from the given buffer.
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

    /// Flushes any cached writes (no-op for now).
    fn flush(&mut self) -> DevResult {
        Ok(())
    }

    /// Returns the total number of blocks available on the device.
    #[inline]
    fn num_blocks(&self) -> u64 {
        self.0.get_block_num()
    }

    /// Returns the block size in bytes.
    #[inline]
    fn block_size(&self) -> usize {
        self.0.get_block_size()
    }
}
