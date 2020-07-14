.macro SAVE reg, offset
    sd  \reg, \offset*8(sp)
.endm

.macro LOAD reg, offset
    ld  \reg, \offset*8(sp)
.endm

.section .text
.globl __interrupt
__interrupt:
    # Swap stack
    csrrw sp, sscratch, sp

    # Kernel mode re-entry if sscratch == 0.
    beq sp, zero, kernel_mode_reentry

    # Otherwise we entered from user mode. `sp` now points to `RawThreadState`.

    # Save original `gp`.
    SAVE x3, 3 # x3 == gp

    # Now use `gp` as a scratch register for usermode stack pointer, and clear `sscratch`.
    csrrw x3, sscratch, zero
    # Save usermode stack pointer.
    SAVE x3, 2

    # Load kernel `gp` from `RawThreadState.hart`.
    LOAD x3, 34

    j interrupt_save_start

kernel_mode_reentry:

    # Swap `sp` back.
    csrrw sp, sscratch, sp

    # Store `sp`.
    SAVE sp, (-36 + 2)

    # Allocate a new `RawThreadState`.
    addi sp, sp, -8*36

    # Fill padding with zero.
    SAVE zero, 35

    # `gp` already contains pointer to the current `HardwareThread`.
    # Store it to the `RawThreadState`.
    SAVE gp, 34

    # ... and the regular `gp` slot.
    SAVE gp, 3

interrupt_save_start:

    SAVE    x0, 0 # Ensure other code sees a zero value.
    SAVE    x1, 1
    # x2 (sp) already saved
    # x3 (gp) already saved
    SAVE    x4, 4 # x4 == tp
    SAVE    x5, 5
    SAVE    x6, 6
    SAVE    x7, 7
    SAVE    x8, 8
    SAVE    x9, 9
    SAVE    x10, 10
    SAVE    x11, 11
    SAVE    x12, 12
    SAVE    x13, 13
    SAVE    x14, 14
    SAVE    x15, 15
    SAVE    x16, 16
    SAVE    x17, 17
    SAVE    x18, 18
    SAVE    x19, 19
    SAVE    x20, 20
    SAVE    x21, 21
    SAVE    x22, 22
    SAVE    x23, 23
    SAVE    x24, 24
    SAVE    x25, 25
    SAVE    x26, 26
    SAVE    x27, 27
    SAVE    x28, 28
    SAVE    x29, 29
    SAVE    x30, 30
    SAVE    x31, 31

    csrr    s1, sstatus
    csrr    s2, sepc
    SAVE    s1, 32
    SAVE    s2, 33

    # &mut RawThreadState
    mv      a0, sp
    # scause: Scause
    csrr    a1, scause
    # stval: usize
    csrr    a2, stval

    jal  handle_interrupt

    # handle_interrupt should never return
    ebreak

.globl leave_interrupt
leave_interrupt:
    # We `LOAD` with `sp` as the base. So store `a0` into `sp`.
    mv sp, a0

    LOAD    s1, 32
    LOAD    s2, 33
    csrw    sstatus, s1
    csrw    sepc, s2

    LOAD    x1, 1
    LOAD    x3, 3
    LOAD    x4, 4
    LOAD    x5, 5
    LOAD    x6, 6
    LOAD    x7, 7
    LOAD    x8, 8
    LOAD    x9, 9
    LOAD    x10, 10
    LOAD    x11, 11
    LOAD    x12, 12
    LOAD    x13, 13
    LOAD    x14, 14
    LOAD    x15, 15
    LOAD    x16, 16
    LOAD    x17, 17
    LOAD    x18, 18
    LOAD    x19, 19
    LOAD    x20, 20
    LOAD    x21, 21
    LOAD    x22, 22
    LOAD    x23, 23
    LOAD    x24, 24
    LOAD    x25, 25
    LOAD    x26, 26
    LOAD    x27, 27
    LOAD    x28, 28
    LOAD    x29, 29
    LOAD    x30, 30
    LOAD    x31, 31

    LOAD    x2, 2
    sret