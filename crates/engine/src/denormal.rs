#[cfg(target_arch = "x86_64")]
const FLUSH: u64 = 0x8040;
#[cfg(target_arch = "aarch64")]
const FLUSH: u64 = 1 << 24;

pub struct NoDenormals {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    saved: u64,
}

impl NoDenormals {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let saved = read();
            write(saved | FLUSH);
            Self { saved }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        Self {}
    }
}

impl Drop for NoDenormals {
    fn drop(&mut self) {
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        write(self.saved);
    }
}

#[cfg(target_arch = "x86_64")]
fn read() -> u64 {
    let mut csr = 0u32;
    unsafe {
        std::arch::asm!("stmxcsr [{}]", in(reg) &raw mut csr, options(nostack, preserves_flags));
    }
    u64::from(csr)
}

#[cfg(target_arch = "x86_64")]
fn write(csr: u64) {
    let csr = csr as u32;
    unsafe {
        std::arch::asm!("ldmxcsr [{}]", in(reg) &raw const csr, options(nostack, readonly, preserves_flags));
    }
}

#[cfg(target_arch = "aarch64")]
fn read() -> u64 {
    let fpcr: u64;
    unsafe {
        std::arch::asm!("mrs {}, fpcr", out(reg) fpcr, options(nomem, nostack, preserves_flags));
    }
    fpcr
}

#[cfg(target_arch = "aarch64")]
fn write(fpcr: u64) {
    unsafe {
        std::arch::asm!("msr fpcr, {}", in(reg) fpcr, options(nomem, nostack, preserves_flags));
    }
}

#[cfg(all(test, any(target_arch = "x86_64", target_arch = "aarch64")))]
mod tests {
    use std::hint::black_box;

    use super::*;

    mod when_the_guard_is_active {
        use super::*;

        #[test]
        fn flushes_denormal_results_to_zero() {
            let _guard = NoDenormals::new();
            assert_eq!(black_box(f32::MIN_POSITIVE) * black_box(0.5), 0.0);
        }
    }

    mod when_the_guard_is_dropped {
        use super::*;

        #[test]
        fn restores_the_previous_mode() {
            drop(NoDenormals::new());
            assert!(black_box(f32::MIN_POSITIVE) * black_box(0.5) > 0.0);
        }
    }
}
