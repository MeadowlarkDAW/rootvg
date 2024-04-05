use std::marker::PhantomData;
use std::ops::Range;

/// A helper struct for a [`wgpu::Buffer`].
pub struct Buffer<T> {
    pub raw: wgpu::Buffer,

    label: &'static str,
    size: u64,
    usage: wgpu::BufferUsages,
    type_: PhantomData<T>,
}

impl<T: bytemuck::Pod> Buffer<T> {
    pub fn new(
        device: &wgpu::Device,
        label: &'static str,
        amount: usize,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let size = next_copy_size::<T>(amount);

        let raw = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        });

        Self {
            label,
            size,
            usage,
            raw,
            type_: PhantomData,
        }
    }

    /// Returns `true` if the buffer was expanded.
    pub fn expand_to_fit_new_size(&mut self, device: &wgpu::Device, new_count: usize) -> bool {
        let new_size = (std::mem::size_of::<T>() * new_count) as u64;

        if self.size < new_size {
            self.raw = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: new_size,
                usage: self.usage,
                mapped_at_creation: false,
            });

            self.size = new_size;

            true
        } else {
            false
        }
    }

    /// Returns the size of the written bytes.
    pub fn write(&mut self, queue: &wgpu::Queue, offset: usize, contents: &[T]) -> usize {
        let offset = offset as u64 * std::mem::size_of::<T>() as u64;

        let bytes: &[u8] = bytemuck::cast_slice(contents);
        queue.write_buffer(&self.raw, offset, bytes);

        bytes.len()
    }

    pub fn slice(&self, range: Range<usize>) -> wgpu::BufferSlice<'_> {
        self.raw.slice(
            range.start as u64 * std::mem::size_of::<T>() as u64
                ..range.end as u64 * std::mem::size_of::<T>() as u64,
        )
    }

    pub fn label(&self) -> &'static str {
        self.label
    }
}

pub fn next_copy_size<T>(amount: usize) -> u64 {
    let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;

    (((std::mem::size_of::<T>() * amount).next_power_of_two() as u64 + align_mask) & !align_mask)
        .max(wgpu::COPY_BUFFER_ALIGNMENT)
}
