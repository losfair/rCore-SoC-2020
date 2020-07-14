# Our entry point.

.section .text.entry
.globl _start
_start:

# Prepare the page table.
la t0, boot_page_table
srli t0, t0, 12 # PhysAddr -> PPN
li t1, (8 << 60) # Sv39
or t0, t0, t1
csrw satp, t0
sfence.vma

# Load boot stack.
li t0, 0xffffffff00000000
la sp, boot_stack_top
or sp, sp, t0

# Calculate virtual address of rust_main.
li t0, 0xffffffff00000000
la t1, rust_main
or t0, t0, t1

# Jump to rust_main.
jr t0

# Boot page table.
.section .data.boot_page_table
boot_page_table:
.quad 0
.quad 0
.quad (0x80000 << 10) | 0xf # Identity mapping.
.zero 507 * 8
.quad (0x80000 << 10) | 0xf # Kernel mapping.
.quad 0

.section .bss.stack
.globl boot_stack
boot_stack:
.space 4096 * 16 # 64 KBytes
.globl boot_stack_top
boot_stack_top: