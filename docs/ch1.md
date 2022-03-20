## Ch1 - 构建最小内核

在本章内容中，我们将开始创建我们的内核，并让它能够成功运行在 RISC-V 裸机上，打印出 `Hello, world!`。

### 创建项目

使用 `cargo` 来创建一个新的项目。

```shell
cargo new rcore
```

### 移除标准库

Rust 的标准库 `std` 的实现是依赖于操作系统平台的，而我们需要编写的内核程序是需要运行在一个无操作系统的裸机上面的，因此需要移除 Rust 默认使用的标准库。而 Rust 提供了一个经过裁剪后的 `core` 库，里面包含了诸如 `Option`，`Result`，以及 `Vec` 等核心类型，其不需要依赖于操作系统，因此我们可以用 `core` 库来替换 `std`。

首先我们指定默认的目标编译平台为 `riscv64gc-unknown-none-elf`。

```toml
# rcore/.cargo/config
[build]
target = "riscv64gc-unknown-none-elf"
```

然后在 `rcore/src/main.rs` 的开头添加 `#![no_std]` 来禁用标准库。之后如果尝试使用 `cargo build` 来构建程序的话，会提示找不到 `println` 宏，因为 `println`，`print`，`panic` 等这几个宏都是在标准库内实现的，现在移除掉标准库后需要我们自己手动实现。

### 实现 panic 宏

创建一个新的 `lang_items` 模块，在其中来实现 `panic` 宏。只需要给指定的处理 `PanicInfo` 的函数一个 `#[panic_handler]` 属性即可。目前这个函数什么也不能干，它的返回类型为 `!`，表示这个函数永远不会产生返回值。

```rust
// rcore/src/lang_items.rs
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
```

### 移除 main 函数

我们一般的程序入口都是一个 `main` 函数，但实际上在程序执行前，标准库还会对 `main` 函数进行一些初始化。而我们现在已经移除了标准库，所以 `main` 函数不能再正常作为程序入口。因此在 `main.rs` 的开头标注上 `#![no_main]` 来禁用 `main` 函数。

### Qemu 启动流程

我们目标将在 Qemu RISC-V virt 平台上运行内核程序，其物理内存的起始地址为 `0x80000000`。

Qemu 的启动流程可以分为三个阶段：

1. 必要的文件被加载到 Qemu 的物理内存之后，Qemu 会在执行几条固化的指令后跳转到物理地址 `0x80000000`，进入第二阶段。
2. 我们需要将 bootloader `rustsbi-qemu.bin` 加载到物理地址 `0x80000000` 开头的物理内存当中，确保第一阶段结束后跳转到的第一条指令即是 bootloader 中的第一条指令。然后当 bootloader 对计算机进行初始化工作后跳转至下一阶段的入口，即将计算机的控制权移交给内核镜像 `rcore.bin`。我们使用的 RUSTSBI 规定的下一阶段入口地址固定为地址 `0x80200000`。
3. 将内核镜像加载到起始物理地址 `0x80200000` 的地方，确保第二阶段结束跳转后即执行的是内核的第一条指令，此时计算机的控制权便已移交给了内核。

### 调整内存布局

#### 程序内存布局

源代码被编译成可执行文件后，其包含的字节主要可以分为代码段和数据段两部分，不同的段会被编译器放置在内存的不同位置上。

一个 C 语言程序从 C 源文件到可执行文件，需要经历**编译器->汇编器->连接器**的处理。编译器负责将 **C 源文件**翻译为**汇编语言**，汇编器负责将汇编指令转化为**机器码**，得到一个**目标二进制文件**，而连接器需要将汇编器生成的目标文件以及一些可能需要的其它外部文件链接在一起形成一个完整的可执行文件。

链接器主要完成两件事情：

1. 将来自不同目标文件中的段在目标文件中进行重新排布。
2. 将变量、函数等符号替换为具体的地址。

#### 编写内核第一条指令

为了便于测试验证我们的内核镜像是否正确对接到 Qemu 上，我们编写一条的指令到内核中。

```asm
# rcore/src/entry.asm
    .section .text.entry
    .globl _start
_start:
    li x1, 100
```

一般情况下，所有的代码都是放在 `.text` 代码段中的，但这里我们使用 `.section` directive 将其放在了一个名为 `.text.entry` 的段中，从而区别于其他的 `.text` 段，确保该段被放在比其它任何代码段更低的地址上。这样，作为内核的入口，这段指令才能被最先执行。

接着使用 Rust 的 `global_asm` 宏将该小段汇编全局嵌入到代码中。

```rust
// rcore/src/main.rs
#![no_std]
#![no_main]

mod lang_item;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));
```

#### 编写链接脚本

链接器默认生成出来的内存布局不符合上述提到的要求，因此我们需要通过链接脚本来调整链接器的行为。编写如下的链接脚本到 `rcore/src/linker.ld`。

```shell
OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}
```

链接脚本的前三行分别设置了目标平台为 `riscv`，设置了整个程序的入口点为符号 `_entry`，定义了常量 `BASE_ADDRESS` 为我们内核镜像需要被放置的起始物理地址。

之后的 `.` 表示当前地址，我们还可以创建一些全局符号例如 `stext` 将其赋值为 `.`，即记录下各段首尾的位置。

可以看到在 `.text : {}` 中，我们将 `.text.entry` 放置在了最开头的位置。

关于链接脚本的更多内容可以参考 [Linker Scripts (LD)](https://sourceware.org/binutils/docs/ld/Scripts.html)。


之后我们需要更改 cargo 的配置，使得它在编译时能够使用我们的链接脚本。

```toml
# rcore/.cargo/config
[build]
target = "riscv64gc-unknown-none-elf"

[target.riscv64gc-unknown-none-elf]
rustflags = [
  "-Clink-arg=-Tsrc/linker.ld", "-Cforce-frame-pointers=yes"
]
```

### 加载内核可执行文件

完成上述操作后，使用 `cargo build --release` 进行构建编译会得到我们的内核可执行文件。但它还不能直接被 Qemu 加载使用，因为这生成出来的文件中除了代码段和数据段外还包含一些元信息，我们需要去除掉其中的元信息。

使用 `rust-objcopy` 工具来生成内核镜像。

```shell
rust-objcopy --strip-all target/riscv64gc-unknown-none-elf/release/rcore -O binary target/riscv64gc-unknown-none-elf/release/rcore.bin
```

### 使用 GDB 调试启动

首先启动 Qemu 模拟器并加载 RustSBI 和内核镜像。

```shell
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=target/riscv64gc-unknown-none-elf/release/rcore.bin,addr=0x80200000 \
    -s -S
```

最后的 `-s` 是让 Qemu 监听本地 TCP 端口 1234 等待 GDB 的连接，`-S` 是让 Qemu 收到 GDB 请求后才开始运行，所以如果想要直接运行 Qemu 的话需要去掉最后一行的参数。

然后启动 gdb 连接进行调试。

```shell
riscv64-elf-gdb \
    -ex 'file target/riscv64gc-unknown-none-elf/release/rcore' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'
```

启动之后先使用 `x/10i $pc` 命令，查看将要执行的 10 条指令，这样可以看到 Qemu 固件中包含的起始指令。

```shell
(gdb) x/10i $pc
=> 0x1000:      auipc   t0,0x0
   0x1004:      addi    a2,t0,40
   0x1008:      csrr    a0,mhartid
   0x100c:      ld      a1,32(t0)
   0x1010:      ld      t0,24(t0)
   0x1014:      jr      t0
   0x1018:      unimp
   0x101a:      0x8000
   0x101c:      unimp
   0x101e:      unimp
```

可以看到其实 Qemu 的固件中只包含 6 条指令（比教程文档里的多了一条，可能是因为 Qemu 版本不同的原因）。

之后使用 `si` 进行单步执行，直到 `jr t0` 这条指令。然后使用 `p/x` 指令将寄存器 `t0` 里的内容以十六进制打印出来。可以看到 `t0` 寄存器里的值是 `0x80000000`，即下一条指令将要跳转至的地址。

```shell
(gdb) si
0x0000000000001004 in ?? ()
(gdb) si
0x0000000000001008 in ?? ()
(gdb) si
0x000000000000100c in ?? ()
(gdb) si
0x0000000000001010 in ?? ()
(gdb) si
0x0000000000001014 in ?? ()
(gdb) p/x $t0
$1 = 0x80000000
```

接下来测试我们的内核能否被正常执行。使用 `b` 指令在 `0x80200000` 处打个断点。然后继续执行到断点处。

```shell
(gdb) b *0x80200000
Breakpoint 1 at 0x80200000
(gdb) c
Continuing.

Breakpoint 1, 0x0000000080200000 in stext ()
```

之后再使用 `x/5i $pc` 便可以成功地查看到我们内核当中的指令，并且可以继续执行看是否正确。使用 `p/d` 以十进制格式将寄存器内容打印出。

```shell
(gdb) x/5i $pc
=> 0x80200000:      li      ra,100
0x80200004: unimp
0x80200006: unimp
0x80200008: unimp
0x8020000a: unimp
(gdb) si
0x0000000080200004 in ?? ()
(gdb) p/d $x1
2 = 100
```

### 编写 Makefile

后续我们会经常用到上面的编译、提取内核镜像、启动 Qemu、启动 gdb 等操作，而这些命令又比较繁琐，因此我们可以编写 Makefile 来简化流程。当然，既然都用上了现代化的 Rust 语言和现代化的 Cargo 工具链，那么当然也要用现代化的 Makefile 啦。（其实是因为我不会写 Makefile）

我们将使用 [cargo-make](https://github.com/sagiegurari/cargo-make)，只需要直接用 `cargo` 安装这个包即可。

```shell
cargo install --force cargo-make
```

然后编辑一个新文件 `rcore/Makefile.toml`，里面就可以编写我们的 task 了，下面是一个 task 最基本的语法。

```toml
[tasks.name]
command = "cmd"
args = [
  "--arg-0",
  "--arg-1"
]
dependencies = ["another"]
```

可以看到我们只需要指定需要执行的命令以及对应参数即可，另外还可以可选地添加一个依赖项，即在执行当前 task 前需要先执行 依赖 task。

这是最终我写的所有的 task

```toml
[tasks.clean]
command = "cargo"
args = ["clean"]

[tasks.build]
command = "cargo"
args = ["build"]
dependencies = ["clean"]

[tasks.release]
command = "cargo"
args = ["build", "--release"]
dependencies = ["clean"]

[tasks.strip]
command = "rust-objcopy"
args = [
  "--strip-all",
  "target/riscv64gc-unknown-none-elf/release/rcore",
  "-O",
  "binary",
  "target/riscv64gc-unknown-none-elf/release/rcore.bin"
]
dependencies = ["release"]

[tasks.debug]
command = "qemu-system-riscv64"
args = [
  "-machine",
  "virt",
  "-nographic",
  "-bios",
  "../bootloader/rustsbi-qemu.bin",
  "-device",
  "loader,file=target/riscv64gc-unknown-none-elf/release/rcore.bin,addr=0x80200000",
  "-s",
  "-S"
]
dependencies = ["strip"]

[tasks.run]
command = "qemu-system-riscv64"
args = [
  "-machine",
  "virt",
  "-nographic",
  "-bios",
  "../bootloader/rustsbi-qemu.bin",
  "-device",
  "loader,file=target/riscv64gc-unknown-none-elf/release/rcore.bin,addr=0x80200000"
]
dependencies = ["strip"]

[tasks.gdb]
command = "riscv64-elf-gdb"
args = [
  "-ex", 
  "file target/riscv64gc-unknown-none-elf/release/rcore",
  "-ex", 
  "set arch riscv:rv64",
  "-ex", 
  "target remote localhost:1234"
]
```

之后需要直接运行 Qemu 的时候只需要在 `rcore` 目录下执行 `cargo make run`，需要调试运行 Qemu 的话只需要执行 `cargo make debug`，然后再使用 `cargo make gdb` 来启动 gdb。

### 函数调用与调用栈

汇编指令在被 CPU 执行时是按照物理地址顺序依次连续向下逐条执行的，缺少在编程语言当中的结构化的控制流（例如 `if else` `while` 等分支结构），只能进行基于指令地址的跳转。

另一种更加复杂的控制流程结构是**函数调用**，同一个函数可能在多个地方被调用，并且在被调用完毕后还需要能够返回到被调用处。要实现函数调用不能只进行简单的跳转，RISC-V 指令集中提供了以下关于函数调用跳转的指令：

| 指令                      | 功能                                             |
| :------------------------ | :----------------------------------------------- |
| $jal\ rd, imm[20:1]$      | $rd \leftarrow pc + 4 \\ pc \leftarrow pc + imm$ |
| $jalr\ rd, (imm[10:0])rs$ | $rd \leftarrow pc + 4 \\ pc \leftarrow rs + imm$ |

其中 `rs` 表示源寄存器，`imm` 表示立即数，`rd` 代表目标寄存器。这两条指令在 `pc` 寄存器中写入将要跳转的指令地址之前，还将当前指令的下一条指令地址保存在了 `rd` 寄存器中（这里假定所有指令长度均为 4 字节）。

在 RISC-V 架构中通常使用 `ra` 寄存器作为目标寄存器，即在函数执行完毕返回的时候，只需要跳转回 `ra` 所保存的地址。

由于我们将返回的地址保存在了 `ra` 寄存器当中，所以在函数执行的过程中该寄存器的值始终不能发生变化，否则就不能跳转返回到函数执行之前的位置。但整个 CPU 仅有一套寄存器，如果按照上面的描述实现函数调用，则不能实现多层函数嵌套调用，因为每产生一次函数调用，`ra` 寄存器中存储的之前的地址就会被覆盖。

像这样的由于函数调用，在控制流转移前后需要保持不变的寄存器的合集称为**函数调用上下文**。我们需要将这些函数调用上下文保存在物理内存上，在当函数调用完毕后从内存当中读取恢复函数调用上下文中的寄存器内容。

这块特定的内存区域则被称作**栈**，用 `sp` 寄存器来保存栈顶指针，在 RISC-V 架构中栈由高地址向低地址增长。在函数被调用的时候，栈中会被分配一块新的内存区域，用来进行函数调用上下文的保存与恢复，这块内存叫做**栈帧**。

#### 分配启动栈

在 `entry.asm` 这一小段入口的汇编内分配启动栈空间，并将栈指针寄存器 `sp` 设置为栈顶的位置，然后将控制权转交给 Rust 的主函数入口。

```shell
    .section .text.entry
    .global _start
_start:
    la sp, boot_stack_top
    call rust_main

    .section .bss.stack
    .global boot_stack
boot_stack:
    .space 4096 * 16
    .global boot_stack_top
boot_stack_top:
```

`.bss` 段包含了静态分配的已声明但没有赋值的变量，我们分配了一块大小为 $4096 * 16$ 字节即 $64 KiB$ 的内存空间用作栈空间，并分配为 `.bss.stack` 段，用**高地址**符号 `boot_stack_top` 来标识栈顶，用**低地址**符号 `boot_stack` 标识栈底。最终 `.bss.stack` 段在链接的过程中会被汇合到 `.bss` 段的最高处。

接下来在 `rcore/src/main.rs` 当中编写新的执行入口，注意需要使用 `#[no_mangle]` 标注，防止编译器对该函数的名字进行混淆，否则在链接的时候就无法找到 `rust_main` 这个符号。

```rust
#[no_mangle]
pub fn rust_main() -> ! {
  loop {}
}
```

我们前面提到 `.bss` 段一般用于放置已声明但未赋值的变量，我们需要给该段的数据初始化为零。

```rust
#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    loop {}
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    ((sbss as usize)..(ebss as usize)).for_each(|b| unsafe { (b as *mut u8).write_volatile(0) });
}
```

在 `clear_bss()` 函数当中的 `extern "C" {}` 中可以引用一个外部的 C 函数接口，这里引用了两个在链接脚本里定义的符号，`sbss` 和 `ebss`，分别表示 `bss` 段的起始地址和结束地址。之后将该区间内的每个地址转为指针 `*mut u8`，写入零值，从而实现了遍历该地址区间并逐字节清零。

### 使用 SBI 服务

RISC-V SBI 全称为 *Supervisor Binary Interface*，提供了访问仅限于机器模式的寄存器的接口，介于底层硬件与内核之间。它除了在计算机启动时负责对环境进行初始化工作之后将计算机控制权移交给内核，实际上还作为内核的执行环境，在内核运行时响应内核的请求为内核提供服务。SBI 有多种实现，OpenSBI 是 RISC-V 官方提供的 C 语言实现参考，我们用到的 RustSBI 是其对应的 Rust 语言实现。

> 关于 SBI 于 BIOS 之间有什么关系，参考这条[评论](https://github.com/rcore-os/rCore-Tutorial-Book-v3/issues/71#issuecomment-869029635)。

新建一个 `sbi` 模块，用以提供 SBI 服务。

```rust
// rcore/src/sbi.rs
#![allow(unused)]

use core::arch::asm;

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;

#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
        "ecall",
        inlateout("x10") arg0 => ret,
        in("x11") arg1,
        in("x12") arg2,
        in("x17") which
        )
    }
    ret
}
```

在 `sbi_call()` 函数实现中，我们内嵌了一段汇编来实现 SBI 服务的调用，同时将调用结果返回。在最上面定义了部分 RustSBI 支持的服务类型的对应的编号，因为目前我们并未用到所有的定义的常量，Rust 在编译时会针对未使用的变量给出警告，暂时使用 `#[allow(unused)]` 标注来取消警告。

之后我们便可以将 SBI 服务封装成函数，例如打印输出字符，关机服务。

```rust
// rcore/src/sbi.rs
pub fn console_putchar(ch: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, ch, 0, 0);
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    panic!("Shutdown failed!");
}
```

### 实现格式化输出宏

现在来实现标准库中的 `print!()` 和 `println!()` 宏，新建一个 `console` 模块。

```rust
// rcore/src/console.rs
use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct StdOut;

impl Write for StdOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.chars().for_each(|ch| console_putchar(ch as usize));
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    StdOut.write_fmt(args).unwrap()
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $(arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}
```

`fmt::Write` 这个 trait 中包含一个 `write_fmt()` 方法，我们借助它来实现 `println!()`。要实现 `fmt::Write` trait 只需要实现 `write_str` 方法即可，这个方法是用于输出一个字符串，我们只需要逐个遍历输出字符即可。

### 完善 panic

现在我们实现了输出功能了，可以完善之前的 `panic()` 实现，使得在产生致命错误的时候能够打印出相关错误信息。

```rust
// rcore/src/lang_items.rs
use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("Pacnicked: {}", info.message().unwrap());
    }
    shutdown()
}
```

之后可以在我们的函数入口 `rust_main()` 里面调用测试一下。

### 练习：实现彩色日志输出

实现彩色输出其实很简单，只需要在对应字符串前加上特定颜色代码的转义字符就行。

```shell
echo -e "\x1b[31mHello, world!\x1b[0m"
```

为了便于实现分级别输出日志，可以使用 `log` 这个 crate，并且这个 `crate` 可以在没有标准库的情况下使用。

要实现一个自己的 Logger 只需要实现 `log::Log` 这个 trait 即可，然后我们就能够使用 `log` 模块内提供的 `info!()` `debug!()` 等宏来进行日志输出。

更加详细的文档可以参考 [log - docs.rs](https://docs.rs/log/0.4.14/log/)，下面放出具体实现：

```rust
// rcore/src/logging.rs
use log::{Level, LevelFilter, Log};

struct Logger;

static LOGGER: Logger = Logger;

pub fn init() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap()
}

impl Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let color = log_level_to_color(record.metadata().level());
            println!(
                "\x1b[{}m[{}] - {}\x1b[0m",
                color,
                record.metadata().level().as_str(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn log_level_to_color(level: log::Level) -> usize {
    match level {
        Level::Error => 31,
        Level::Debug => 32,
        Level::Info => 34,
        Level::Warn => 93,
        Level::Trace => 90,
    }
}
```

之后我们便可以在入口函数当中使用 logger。

```rust
// rcore/src/main.rs
#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    println!("Hello World!");
    log::info!("test");
    log::error!("error!");
    sbi::shutdown();
}
```

运行效果如下图

![image](https://user-images.githubusercontent.com/10807119/159147490-05613fc0-6d46-4726-a3e4-bbb94d8ec20c.png)
