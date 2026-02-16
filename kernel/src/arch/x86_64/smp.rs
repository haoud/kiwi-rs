use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::{arch::x86_64, config::MAX_CPUS};

/// Request the bootloader to provide information about the system's CPUs, and
/// to start them up. This will make our life way easier since we just have to
/// provide a function pointer to the entry point of the CPU, and the AP will
/// just start executing code from there.
static LIMINE_SMP_REQUEST: limine::request::MpRequest = limine::request::MpRequest::new();

/// The number of CPUs available in the system. This is used to determine how
/// many CPUs can be used by the kernel. This is initially set to 1, since the
/// BSP is always available, and will be incremented by the APs when they
/// start up.
/// Unresponsive APs will not increment this counter and will not be used by the
/// kernel.
static CPU_AVAILABLE: AtomicUsize = AtomicUsize::new(1);

/// Whether the APs have started up and are initialized.
static AP_READY: AtomicBool = AtomicBool::new(false);

/// Setup the SMP environment.
///
/// # Panics
/// Panics if the bootloader does not provide SMP support.
pub fn setup() {
    let smp_response = LIMINE_SMP_REQUEST
        .get_response()
        .expect("No SMP support provided by the bootloader");
    let cpu_count = smp_response.cpus().len();

    // Some assertions to ensure that the bootloader provided valid information
    // about the CPUs in the system, and that the kernel is correctly
    // configured to fit the number of CPUs in the system into an u8.
    assert!(cpu_count > 0, "No CPU detected");
    assert!(
        u8::try_from(MAX_CPUS).is_ok(),
        "x86_64 architecture only supports up to 255 CPUs"
    );
    assert!(
        cpu_count <= MAX_CPUS,
        "Too many CPUs detected: {cpu_count} (max: {MAX_CPUS})",
    );

    // Start the auxiliary processors by writing the entry point to the
    // APs' goto address. The APs will start executing code from this
    // address immediately after this change.
    smp_response
        .cpus()
        .iter()
        .filter(|cpu| cpu.lapic_id != smp_response.bsp_lapic_id())
        .for_each(|cpu| {
            cpu.goto_address.write(x86_64::ap_start);
        });

    // Wait for the auxiliary processors to start up. We need to wait for them
    // to start up because during the initialization of the kernel, IPIs can be
    // sent to the APs, and if they haven't be initialized yet, this may cause
    // the APs to crash.
    // TODO: Add a timeout here to avoid waiting indefinitely if an AP
    // fails to start up.
    while CPU_AVAILABLE.load(Ordering::Relaxed) < cpu_count {
        core::hint::spin_loop();
    }
    AP_READY.store(true, Ordering::Release);
}

/// Setup the auxiliary processor.
pub fn ap_setup() {
    CPU_AVAILABLE.fetch_add(1, Ordering::Relaxed);
}

/// Check if the APs have started up and are initialized.
#[must_use]
pub fn ap_ready() -> bool {
    AP_READY.load(Ordering::Acquire)
}

/// Get the number of CPUs available in the system. This includes the BSP and
/// the APs that have started up and are initialized.
#[must_use]
pub fn cpu_count() -> usize {
    CPU_AVAILABLE.load(Ordering::Relaxed)
}

/// Get the identifier of the current CPU. Each CPU has a unique identifier
/// that can be used to differentiate them.
#[must_use]
pub fn cpu_identifier() -> u8 {
    todo!()
}
