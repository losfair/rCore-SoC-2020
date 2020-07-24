# rCore-Tutorial 实验报告

## 学习路径

由于先前有 Rust 语言的开发经验，也对 RISC-V 用户模式指令集有一点了解，因此直接开始按照 lab tutorials 的路径手写代码实现其所描述的 OS .

在实现过程中有时会遇到一些不熟悉的概念，通过查 RISC-V 特权指令集手册解决。

完全重新实现的部分为 Lab 0 - Lab 4 ，最后两章由于时间不够，选择用做实验题的方式学习。

## 完成的功能

- 手写代码从头实现 Lab 0-4 的内容
- 完成 Lab 6 的实验题
- 基础的 SMP 支持
- 基于等待队列的 futex-like 同步互斥机制

## Part 1. 手写代码从头实现 Lab 0-4 的内容

我先前有开发 x86-64 微内核的经历，因此这一部分会包含这些方面的对比：

- x86-64 vs RISC-V
- 宏内核 vs 微内核

### Lab 0

Lab 0 的内容是环境配置和项目初始化。与 x86-64 架构相比，感觉 RISC-V 的内核编译和前期启动流程要简单许多：不需要实现处理器模式的切换 (real mode -> long mode) ，所有代码均为 64 位，使用标准的 ELF 工具即可得到可加载的内核镜像；也不需要处理与 BIOS / UEFI / multiboot 等不同 bootloader 环境之间复杂的接口问题。

SBI 接口有点类似于 x86 的 BIOS 中断和 UEFI services ，但是它也提供 IPI 这种 OS 需要一直使用的功能，调用方式对开发者来说也似乎更友好一点。

### Lab 1

Lab 1 的内容是中断机制和中断处理。作为现代指令集架构，感觉 RISC-V 的中断机制有它应有的优雅。

时钟和时钟中断方面，与 x86 架构 PIC / APIC / `rdtsc` 等历史遗留的混乱相比，RISC-V 的 time 寄存器和 SBI set_timer 显得十分简单。

### Lab 2

Lab 2 的内容是内核堆和物理内存的动态分配。

我对这一章功能的实现方法与教程文档有一些差异：

1. 教程选择分开内核堆和其他物理页的物理内存范围，而在我的实现中物理页面直接从内核堆中动态分配。具体实现见 `memory/pool.rs: PagePool` 。
2. 关于内核堆的分配器，教程选择 `buddy_system_allocator` ，我的实现选用 [dlmalloc 的 Rust 移植](https://github.com/alexcrichton/dlmalloc-rs) 。为适配内核环境，我对 dlmalloc-rs [做了一些修改](https://github.com/losfair/dlmalloc-rs/tree/rcore-soc) 。

动态分配机制是我在实现 rCore-Tutorial OS 中发现的这个宏内核 OS 与微内核的第一个主要区别。内存分配器作为一种“策略” (policy) 而非“机制” (mechanism) 在许多微内核如 [seL4](https://github.com/seL4/seL4) 和我的 [FlatMk](https://github.com/losfair/FlatMk-v0) 中并不存在。但在宏内核中，由于内核数据结构的复杂性，分配器是必要的。

注意到 Rust 中需要堆分配的容器如 `Box` / `Vec` 等并不支持指定分配器。对于内核中的特定情形（如中断上下文等），若要使用动态内存，则指定分配器是必要的。Rust 社区有关于这个问题的提案：https://github.com/rust-lang/wg-allocators/issues/12 .

### Lab 3

Lab 3 的教程描述了虚拟内存机制。页表的结构与 x86-64 没有太多差异，不过默认支持 1GB / 2MB huge pages 的设计很现代了。

### Lab 4

Lab 4 是关于进程、线程及其调度。本章是我花费最多时间的一章，实现中遇到的主要关键点是：

1. 安全地区分线程上下文和中断上下文
2. 高效的锁机制，当锁被占用时让出 CPU 时间而非自旋等待
3. 各种死锁的调试
4. SMP 支持

对于第一点，我的做法是利用 Rust 类型系统保证尽可能多的安全性，且将无法用类型表达安全性的代码路径标为 `unsafe` 。当进入线程上下文或中断上下文时，在这个上下文的入口构造一个 `ThreadToken` / `InterruptToken` (均为 ZST)，对上下文有要求的特定函数在参数中接受这两个 Token 之一。

对于第二点，我实现了等待队列和类似于 Linux futex 的“等待地址-唤醒地址”机制，然后在这上面实现了 Mutex ，并写了一个测试。

第三点死锁的调试似乎没什么技巧。类似于 https://github.com/BurtonQin/rust-lock-bug-detector 的静态分析器可能有帮助，但是这次没有试用。

在第四点 SMP 支持上，我实现了多核的启动和每核心上的线程调度，但是需要 IPI 的功能（线程迁移、跨核 Mutex 解锁唤醒等）还没有实现。

在做 Lab 4 的过程中，我发现 Rust 类型系统用以描述并行的部分似乎不足以完全表达内核所需的所有语义。Rust 类型系统表达并行的标记有 `Send` 和 `Sync` 。在通常的用户程序环境中，它们的语义是很清晰的：

- 满足 `Send` 的类型可以被任一线程独占使用。
- 满足 `Sync` 的类型可以被多个线程同时使用。

然而在内核环境中会遇到一些问题：

- 一个类型可在 CPU 核心间 `Send`/`Sync` 和可在不同软件线程间 `Send`/`Sync` 的语义是不同的
- 没有 trait 标识一个类型是否可被重入访问（异步中断安全性，类似于 Linux 用户态中的 async signal safety）

