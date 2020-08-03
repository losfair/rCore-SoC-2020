# Daily Schedule @ Part 1

*Note: 各部分成果位于 [Report.md](Report.md) 中，此处仅作最简过程记录*

## Day 1 (2020/7/12)

- 阅读 RISC-V 特权指令集文档，准备开始

## Day 2 (2020/7/13)

- 修改 dlmalloc-rs 以支持裸机环境

[losfair/dlmalloc-rs:rcore-soc](https://github.com/losfair/dlmalloc-rs/tree/rcore-soc)

- 完成 Lab 0, 1, 2

[2d07d52a Lab 0](https://github.com/losfair/rCore-SoC-2020/commit/2d07d52abab7f486bf4de78f7984371db9c71d85)

[2a644ae5 Lab 1](https://github.com/losfair/rCore-SoC-2020/commit/2a644ae51ce821c0e86966bd49e50abed42ed760)

[34008000 Lab 2](https://github.com/losfair/rCore-SoC-2020/commit/34008000720500f055e775f6afd33e9d26b8b0e2)

## Day 3 (2020/7/14)

- 完成 Lab 3, 部分完成 Lab 4

[168549ea Lab 3](https://github.com/losfair/rCore-SoC-2020/commit/168549ea20a6b3692f25716e53255d3c04edb7eb)

[52703828 Lab 4](https://github.com/losfair/rCore-SoC-2020/commit/527038286168aceac7f7516d1dac478cfc2c672c)

## Day 4 (2020/7/15)

- 完成 Lab 4
- 实现内核抢占

[b7e8badb Kernel preemption.](https://github.com/losfair/rCore-SoC-2020/commit/b7e8badb7321a7fd9397f6a98dae6b633733b701)

- 利用 Rust 类型系统保证中断重入安全性

## Day 5 (2020/7/16)

- 实现等待队列

[1c340ef5 Wait queue & mutex.](https://github.com/losfair/rCore-SoC-2020/commit/1c340ef5eeee05b8515b8804829e476b6f4ceed7)

- 重构调度器

[d878b077 Run scheduler in thread context.](https://github.com/losfair/rCore-SoC-2020/commit/d878b077b55ea75648e33bd590e41a77dc430eba)

- 某处存在死锁，需解决

## Day 6 (2020/7/17)

- 解决死锁

[5da8a4d0 Fix deadlock](https://github.com/losfair/rCore-SoC-2020/commit/5da8a4d0dd16924e5eb5d28df05cee2831fde991)

- 全局分配器锁

[bbace51c Concurrency fixes.](https://github.com/losfair/rCore-SoC-2020/commit/bbace51c1939c3d78c55c8a03feb821e057d7404)

- 改进调度机制

[4699466c Critical and non-critical scheduling contexts.](https://github.com/losfair/rCore-SoC-2020/commit/4699466c51c5802ee2af82002410bb8f7471a5ce)

## Day 7 (2020/7/18)

- 周末休息

## Day 8 (2020/7/19)

- 周末休息
- 优化代码结构

## Day 9 (2020/7/20)

- 开始实现 SMP

[ade393df Start implementing SMP.](https://github.com/losfair/rCore-SoC-2020/commit/ade393dff127f8907d03395f34c23ce3d1694a09)

## Day 10 (2020/7/21)

- 修复 AP 启动问题

[2a75a2d6 Fix AP boot loop.](https://github.com/losfair/rCore-SoC-2020/commit/2a75a2d66740246c38a0ad0857b042b4a8f5eee6)

## Day 11 (2020/7/22)

Nothing

## Day 12 (2020/7/23)

- 项目代码整体 Review

## Day 13 (2020/7/24)

- 写报告

[4ee0d982 Add report.](https://github.com/losfair/rCore-SoC-2020/commit/4ee0d9827cef8cfb4fa23775d84275d8a235d8bd)

## Day 14 (2020/7/25)

- 完成 Lab 6

[losfair/rCore-Tutorial:lab6](https://github.com/losfair/rCore-Tutorial/tree/lab6)

- 继续写报告

[201b9b42 Update report.](https://github.com/losfair/rCore-SoC-2020/commit/201b9b42eb1fa5327a6b28742acd2bec12672a28)

# Daily Schedule @ Part 2

## Day 1 (2020/8/3)

- 提出在 seL4 上运行 zCore 的选题
