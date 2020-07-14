OUTPUT_ARCH(riscv)

ENTRY(_start)

BASE_ADDRESS = 0xffffffff80200000; /* Virtually mapped to higher half */

SECTIONS
{
    . = BASE_ADDRESS;

    . = ALIGN(4K);

    KERNEL_START = .;

    TEXT_START = .;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4K);

    RODATA_START = .;

    .rodata : {
        *(.rodata .rodata.*)
    }

    . = ALIGN(4K);

    DATA_START = .;

    .data : {
        *(.data.boot_page_table)
        *(.data .data.*)
    }

    . = ALIGN(4K);

    BSS_START = .;

    .bss : {
        *(.sbss .bss .bss.*)
    }

    . = ALIGN(4K);

    KERNEL_END = .;
}