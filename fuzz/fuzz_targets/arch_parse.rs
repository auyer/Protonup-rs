#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::architecture::{CpuArch, arch_from_file_name, strip_arch_suffix};

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);

    // Both supported architectures must survive a Display/FromStr round trip.
    for arch in [CpuArch::X86, CpuArch::Arm] {
        assert_eq!(
            arch.to_string().parse::<CpuArch>(),
            Ok(arch),
            "CpuArch Display/FromStr round trip failed for {arch:?}"
        );
    }

    let parsed = input.parse::<CpuArch>();
    if let Ok(arch) = parsed {
        // CpuArch parsing is ASCII case-insensitive.
        assert_eq!(
            input.to_ascii_uppercase().parse::<CpuArch>(),
            Ok(arch),
            "uppercase variant of an accepted arch must parse identically"
        );
    }

    // `arch_from_file_name` is exactly the arch half of `strip_arch_suffix`.
    // This is the invariant that guards the byte-indexed slicing in
    // `find_arch_token` / `strip_arch_suffix` for arbitrary (non-ASCII) names.
    assert_eq!(
        arch_from_file_name(&input),
        strip_arch_suffix(&input).1,
        "arch_from_file_name and strip_arch_suffix disagree for {input:?}"
    );
});
