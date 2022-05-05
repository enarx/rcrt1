// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]
#![no_std]
#![deny(clippy::all)]
#![deny(missing_docs)]

const R_TYPE_MASK: u64 = 0x7fffffff;

use goblin::elf::dynamic::dyn64::Dyn;
use goblin::elf::dynamic::{DT_REL, DT_RELA, DT_RELASZ, DT_RELSZ};
use goblin::elf::reloc::reloc64::Rel;
use goblin::elf::reloc::reloc64::Rela;
use goblin::elf::reloc::R_X86_64_RELATIVE;

/// Dynamic relocation for a static PIE
///
/// This is normally called early in the _start function:
///     # %rdi - address of _DYNAMIC section
///     # %rsi - base load offset
///     mov    BASE,                    %rsi
///     lea    _DYNAMIC(%rip),          %rdi
///     call   _dyn_reloc
///
/// C version: https://git.musl-libc.org/cgit/musl/tree/ldso/dlstart.c
///
/// # Safety
///
/// This function is unsafe, because the caller has to ensure the dynamic section
/// points to the correct memory.
#[inline(never)]
pub unsafe extern "C" fn dyn_reloc(dynamic_section: *const u64, base: u64) {
    inner_dyn_reloc(dynamic_section, base)
}

#[inline(always)]
unsafe fn inner_dyn_reloc(dynamic_section: *const u64, base: u64) {
    let mut dt_rel: Option<u64> = None;
    let mut dt_relsz: usize = 0;
    let mut dt_rela: Option<u64> = None;
    let mut dt_relasz: usize = 0;

    let mut dynv = dynamic_section as *const Dyn;

    loop {
        match (*dynv).d_tag {
            0 => break,
            DT_REL => dt_rel = Some((*dynv).d_val),
            DT_RELSZ => dt_relsz = (*dynv).d_val as usize / core::mem::size_of::<Rel>(),
            DT_RELA => dt_rela = Some((*dynv).d_val),
            DT_RELASZ => dt_relasz = (*dynv).d_val as usize / core::mem::size_of::<Rela>(),
            _ => {}
        }
        dynv = dynv.add(1);
    }

    if let Some(dt_rel) = dt_rel {
        let rels = core::slice::from_raw_parts((base + dt_rel) as *const Rel, dt_relsz);

        rels.iter()
            .filter(|rel| rel.r_info & R_TYPE_MASK == R_X86_64_RELATIVE as u64)
            .for_each(|rel| {
                let rel_addr = (base + rel.r_offset) as *mut u64;
                rel_addr.write(rel_addr.read() + base);
            });
    }

    if let Some(dt_rela) = dt_rela {
        let relas = core::slice::from_raw_parts((base + dt_rela) as *const Rela, dt_relasz);

        relas
            .iter()
            .filter(|rela| rela.r_info & R_TYPE_MASK == R_X86_64_RELATIVE as u64)
            .for_each(|rela| {
                let rel_addr_0 = (base + rela.r_offset) as *mut u64;
                rel_addr_0.write((base as i64 + rela.r_addend) as u64);
            });
    }
}

/// rcrt1.o replacement
///
/// This function searches the AUX entries in the initial stack `sp`, which are after
/// the argc/argv entries. It uses the AUX entries `AT_PHDR`, `AT_PHENT` and `AT_PHNUM`
/// to search its own elf headers for the `_DYNAMIC` header. With the address of the
/// `_DYNAMIC` header and the value of the `_DYNAMIC` symbol in `dynv` the offset can be
/// calculated from the elf sections to the real address the elf binary was loaded to and
/// `rcrt1::dyn_reloc()` can be called to correct the global offset tables.
///
/// Because the global offset tables are not yet corrected, this function is very fragile.
/// No function (even rust's internal `memset()` for variables initialization) which requires
/// lookup in the global offset tables is allowed. Therefore `#![no_builtins]` is specified.
///
/// # Safety
///
/// This function is unsafe, because the caller has to ensure the stack pointer passed is setup correctly.
#[inline(never)]
pub unsafe extern "C" fn rcrt(
    dynv: *const u64,
    sp: *const usize,
    pre_main: extern "C" fn() -> !,
) -> ! {
    use goblin::elf64::program_header::{ProgramHeader, PT_DYNAMIC};
    const AT_PHDR: usize = 3;
    const AT_PHENT: usize = 4;
    const AT_PHNUM: usize = 5;

    // skip the argc/argv entries on the stack
    let argc: usize = *sp;
    let argv = sp.add(1);
    let mut i = argc + 1;
    while *argv.add(i) != 0 {
        i += 1;
    }
    let auxv_ptr = argv.add(i + 1);

    // search the AUX entries
    let mut phnum: usize = 0;
    let mut phentsize: usize = 0;
    let mut ph: usize = 0;

    let mut i = 0;
    while *auxv_ptr.add(i) != 0 {
        match *auxv_ptr.add(i) {
            AT_PHDR => ph = *auxv_ptr.add(i + 1),
            AT_PHENT => phentsize = *auxv_ptr.add(i + 1),
            AT_PHNUM => phnum = *auxv_ptr.add(i + 1),
            _ => {}
        }
        if ph != 0 && phentsize != 0 && phnum != 0 {
            // found all we need
            break;
        }
        i += 2;
    }

    let mut ph = ph as *const ProgramHeader;
    let mut i = phnum;

    while i != 0 {
        // Search all ELF program headers for the `_DYNAMIC` section
        if (*ph).p_type == PT_DYNAMIC {
            // calculate the offset, where the elf binary got loaded
            let base = dynv as u64 - (*ph).p_vaddr;

            inner_dyn_reloc(dynv, base);

            // Now call the `pre_main()` function and never return
            pre_main()
        }
        ph = (ph as usize + phentsize) as *const ProgramHeader;
        i -= 1;
    }

    // Fail horribly, if we ever reach this point
    unreachable!();
}

/// Macro for the _start entry point
///
/// # Examples
///
/// ```rust,ignore
/// #![no_std]
/// #![no_main]
/// #![feature(naked_functions, asm_sym)]
///
/// use x86_64_linux_nolibc as std;
///
/// use std::println;
/// use std::process::{exit, Termination};
///
/// rcrt1::x86_64_linux_startup!(
///     fn _start() -> ! {
///         exit(main().report().to_i32())
///     }
/// );
///
/// #[panic_handler]
/// fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
///     exit(255)
/// }
///
/// fn main() -> Result<(), i32> {
///     println!("Hello World!");
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! x86_64_linux_startup {
    (fn $name:ident() -> ! $code:block ) => {
        #[no_mangle]
        #[naked]
        pub unsafe extern "sysv64" fn $name() -> ! {
            use core::arch::asm;

            fn inner() -> ! {
                $code
            }

            // Call `rcrt1::rcrt` with the absolute address of the `_DYNAMIC` section
            // and the stack pointer and our `pre_main` function
            asm!(
                "lea    rdi, [rip + _DYNAMIC]",
                "mov    rsi, rsp",
                "lea    rdx, [rip + {INNER}]",
                "jmp   {RCRT}",

                RCRT = sym $crate::rcrt,
                INNER = sym inner,
                options(noreturn)
            )
        }
    };
}
