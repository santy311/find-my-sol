use anyhow::{anyhow, Result};
use ocl::{enums::DeviceInfo, Buffer, Context, Device, Kernel, MemFlags, Platform, Program, Queue};
use std::collections::HashMap;

pub struct OpenCLManager {
    platforms: Vec<Platform>,
    devices: Vec<Device>,
    contexts: HashMap<usize, Context>,
    queues: HashMap<usize, Queue>,
}

impl OpenCLManager {
    pub fn new() -> Result<Self> {
        let platforms = Platform::list();
        let mut devices = Vec::new();
        let mut contexts = HashMap::new();
        let mut queues = HashMap::new();

        for (_platform_idx, platform) in platforms.iter().enumerate() {
            let platform_devices = Device::list(platform, None)?;
            for (_device_idx, device) in platform_devices.iter().enumerate() {
                let global_idx = devices.len();
                devices.push(*device);

                let context = Context::builder()
                    .platform(*platform)
                    .devices(*device)
                    .build()?;

                let queue = Queue::new(&context, *device, None)?;

                contexts.insert(global_idx, context);
                queues.insert(global_idx, queue);
            }
        }

        Ok(OpenCLManager {
            platforms,
            devices,
            contexts,
            queues,
        })
    }

    pub fn list_devices(&self) -> Result<()> {
        println!("Available OpenCL devices:");
        println!("==========================");

        for (idx, device) in self.devices.iter().enumerate() {
            let name = device.info(DeviceInfo::Name)?.to_string();
            let vendor = device.info(DeviceInfo::Vendor)?.to_string();
            let compute_units = device
                .info(DeviceInfo::MaxComputeUnits)?
                .to_string()
                .parse::<u32>()
                .unwrap_or(0);
            let global_mem = device
                .info(DeviceInfo::GlobalMemSize)?
                .to_string()
                .parse::<u64>()
                .unwrap_or(0);
            let local_mem = device
                .info(DeviceInfo::LocalMemSize)?
                .to_string()
                .parse::<u64>()
                .unwrap_or(0);

            println!("Device {}: {}", idx, name);
            println!("  Vendor: {}", vendor);
            println!("  Compute Units: {}", compute_units);
            println!("  Global Memory: {} MB", global_mem / 1024 / 1024);
            println!("  Local Memory: {} KB", local_mem / 1024);
            println!();
        }

        Ok(())
    }

    pub fn get_device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn get_device(&self, idx: usize) -> Result<&Device> {
        self.devices
            .get(idx)
            .ok_or_else(|| anyhow!("Device index {} not found", idx))
    }

    pub fn get_context(&self, idx: usize) -> Result<&Context> {
        self.contexts
            .get(&idx)
            .ok_or_else(|| anyhow!("Context for device {} not found", idx))
    }

    pub fn get_queue(&self, idx: usize) -> Result<&Queue> {
        self.queues
            .get(&idx)
            .ok_or_else(|| anyhow!("Queue for device {} not found", idx))
    }

    pub fn create_vanity_kernel(&self, device_idx: usize) -> Result<VanityKernel> {
        let _device = self.get_device(device_idx)?;
        let context = self.get_context(device_idx)?;
        let queue = self.get_queue(device_idx)?;

        let kernel_source = include_str!("kernels/vanity.cl");

        let program = Program::builder().src(kernel_source).build(context)?;

        let kernel = Kernel::builder()
            .program(&program)
            .name("generate_vanity_keys")
            .global_work_size(1024 * 1024) // 1M work items
            .arg(None::<&Buffer<u32>>) // seed buffer
            .arg(None::<&Buffer<u8>>) // result buffer
            .arg(None::<&Buffer<u8>>) // starts_with_pattern buffer
            .arg(0u32) // starts_with_len
            .arg(None::<&Buffer<u8>>) // ends_with_pattern buffer
            .arg(0u32) // ends_with_len
            .arg(0u32) // case_sensitive
            .build()?;

        Ok(VanityKernel {
            kernel,
            queue: queue.clone(),
            context: context.clone(),
        })
    }
}

pub struct VanityKernel {
    kernel: Kernel,
    queue: Queue,
    context: Context,
}

impl VanityKernel {
    pub fn generate_keys(
        &self,
        seeds: &[u32],
        starts_with: &str,
        ends_with: &str,
        case_sensitive: bool,
    ) -> Result<Vec<u8>> {
        let seed_buffer = Buffer::<u32>::builder()
            .queue(self.queue.clone())
            .flags(MemFlags::new().read_only().copy_host_ptr())
            .len(seeds.len())
            .copy_host_slice(seeds)
            .build()?;

        let result_buffer = Buffer::<u8>::builder()
            .queue(self.queue.clone())
            .flags(MemFlags::new().write_only())
            .len(seeds.len() * 64) // 32 bytes for public key + 32 bytes for private key
            .build()?;

        // Always allocate at least 1 byte for pattern buffers
        let starts_with_bytes = if starts_with.is_empty() {
            &[0u8][..]
        } else {
            starts_with.as_bytes()
        };
        let starts_with_buffer = Buffer::<u8>::builder()
            .queue(self.queue.clone())
            .flags(MemFlags::new().read_only().copy_host_ptr())
            .len(starts_with_bytes.len())
            .copy_host_slice(starts_with_bytes)
            .build()?;

        let ends_with_bytes = if ends_with.is_empty() {
            &[0u8][..]
        } else {
            ends_with.as_bytes()
        };
        let ends_with_buffer = Buffer::<u8>::builder()
            .queue(self.queue.clone())
            .flags(MemFlags::new().read_only().copy_host_ptr())
            .len(ends_with_bytes.len())
            .copy_host_slice(ends_with_bytes)
            .build()?;

        // Set kernel arguments
        self.kernel.set_arg(0, &seed_buffer)?;
        self.kernel.set_arg(1, &result_buffer)?;
        self.kernel.set_arg(2, &starts_with_buffer)?;
        self.kernel.set_arg(3, &(starts_with.len() as u32))?;
        self.kernel.set_arg(4, &ends_with_buffer)?;
        self.kernel.set_arg(5, &(ends_with.len() as u32))?;
        self.kernel
            .set_arg(6, &(if case_sensitive { 1u32 } else { 0u32 }))?;

        // Execute kernel
        unsafe {
            self.kernel.enq()?;
        }

        let mut results = vec![0u8; seeds.len() * 64];
        result_buffer.read(&mut results).enq()?;

        Ok(results)
    }
}
