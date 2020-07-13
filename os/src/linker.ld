OUTPUT_ARCH(riscv)

ENTRY(_start)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;

    KERNEL_START = .;

    TEXT_START = .;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    RODATA_START = .;

    .rodata : {
        *(.rodata .rodata.*)
    }

    DATA_START = .;

    .data : {
        *(.data .data.*)
    }

    BSS_START = .;

    .bss : {
        *(.sbss .bss .bss.*)
    }

    KERNEL_END = .;
}