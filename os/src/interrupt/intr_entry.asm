.macro SAVE reg, offset
    sd  \reg, \offset*8(sp)
.endm

.macro LOAD reg, offset
    ld  \reg, \offset*8(sp)
.endm

.macro SAVE_GREGS_NO_SP_GP
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
    LOAD x3, (34 * 2 + 0)

    j interrupt_save_start

kernel_mode_reentry:
    # Kernel mode reentry accepts a `&mut Context` that is not part of a `RawThreadState`.
    # Swap `sp` back.
    csrrw sp, sscratch, sp

    # Store `sp`.
    SAVE sp, (-34 + 2)

    # Allocate a new `Context`.
    addi sp, sp, -8*34

    # Store `gp`.
    SAVE gp, 3

interrupt_save_start:
    SAVE_GREGS_NO_SP_GP

    csrr    a0, sstatus
    csrr    a1, sepc
    SAVE    a0, 32
    SAVE    a1, 33

    # &mut RawThreadState
    mv      a0, sp
    # scause: Scause
    csrr    a1, scause
    # stval: usize
    csrr    a2, stval

    jal  handle_interrupt

    # handle_interrupt should never return
    ebreak

.globl save_gregs_assuming_intr_disabled
save_gregs_assuming_intr_disabled:
    sd x2, 2*8(a0) # sp
    mv sp, a0 # &mut Context
    SAVE x3, 3 # gp

    li a0, 0 # the "restore path" return value

    SAVE_GREGS_NO_SP_GP # GPR 0 to 31

    SAVE zero, 32 # dummy value for sstatus

    la a0, save_gregs_assuming_intr_disabled__ret
    SAVE a0, 33 # sepc

    LOAD sp, 2 # restore sp

    li a0, 1 # the "save path" return value

save_gregs_assuming_intr_disabled__ret: # shared return path
    ret

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