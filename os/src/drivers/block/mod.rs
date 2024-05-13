//! virtio_blk device driver

mod virtio_blk;
mod block_dev;

pub use virtio_blk::VirtIOBlock;

use alloc::sync::Arc;
pub use block_dev::{BlockDevice, BLOCK_SZ};
use lazy_static::*;

type BlockDeviceImpl = virtio_blk::VirtIOBlock;

// 在 qemu 上，我们使用 VirtIOBlock 访问 VirtIO 块设备，并将它全局实例化为 BLOCK_DEVICE ，使内核的其他模块可以访问。
lazy_static! {
    /// The global block device driver instance: BLOCK_DEVICE with BlockDevice trait
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}

#[allow(unused)]
/// Test the block device
pub fn block_device_test() {
    let block_device = BLOCK_DEVICE.clone();
    let mut write_buffer = [0u8; 512];
    let mut read_buffer = [0u8; 512];
    for i in 0..512 {
        for byte in write_buffer.iter_mut() {
            *byte = i as u8;
        }
        block_device.write_block(i as usize, &write_buffer);
        block_device.read_block(i as usize, &mut read_buffer);
        assert_eq!(write_buffer, read_buffer);
    }
    println!("block device test passed!");
}
