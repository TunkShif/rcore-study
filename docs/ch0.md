## Ch0 - 操作系统概述

### 课程相关

[rCore 教程][0]主要目的在于一步一步展示如何从零开始用 Rust 编写一个 RISC-V 架构上的类 Unix 内核，教程的每一章都是基于之前的章节演示如何一步步的完善最终的 OS，然后每一章节后面都有一些相应的问答题与编程实践题。

对于不想从零开始实现的可以选择做对应的 [rCore Labs][1]，这里面提供了各个一章节完成后的基础代码，完成 Lab 只需要在给出的代码基础上添加新功能即可。但目前看上去这个文档还不是很完善，所以我选择跟着上面的文档从零开始实现。

### 环境配置

因为个人日常使用的是 ArchLinux，对于环境配置来说很容易，没遇到什么问题。

首先是安装部分 C 语言的开发环境，这里只安装了 RISC-V 下的二进制工具和 gdb 调试器，并没有安装 gcc。

```sh
sudo pacman -S riscv64-elf-binutils riscv64-elf-gdb
```

然后配置 Rust 相关的工具链，推荐从官方源里安装 `rustup`，然后使用 `rustup` 来管理不同版本的 `rustc` 编译器。默认安装的是 `stable` 分支的编译器，完成该实验需要使用 `nightly` 分支的编译器。

```sh
rustup install nightly
```

因为我个人可能平时会用到 Rust 进行开发，所以我不建议将全局的默认编译器切换成 `nightly` 分支，而是选择仅将编写实验 OS 的仓库指定为使用 `nightly` 分支。

之后安装 RISC-V 目标架构的相关组件以及一些开发调试工具。

```sh
rustup target add riscv64gc-unknown-none-elf
cargo install cargo-binutils --vers =0.3.3
rustup component add llvm-tools-preview
rustup component add rust-src
```

之后安装 qemu 模拟器，除了安装 `qemu` 软件包本地之外，为了获取 RISC-V 架构的支持，还需要安装 `qemu-arch-extra` 包。

然后可以将教程仓库中已经实现好的 OS 克隆下来，进行运行测试。

```sh
git clone https://github.com/rcore-os/rCore-Tutorial-v3.git
cd rCore-Tutorial-v3/os
make run
```

### 熟悉 GDB 的使用

以练习题 `* 在Linux环境下编写一个会产生异常的应用程序，并简要解释操作系统的处理结果。` 为例子，先编写一个如下的 C 语言程序：

```c
// exception.c
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char *argv[]) {
  int *p = NULL;
  printf("*p = %d", *p);
  return 0;
}
```

使用 gcc 进行编译的时候需要注意使用 `-g` 参数来保留调试需要的符号表信息。

```sh
gcc -g -o exception exception.c
```

然后使用 `gdb ./exception` 进行调试。使用 `run` 来执行程序，可以看到程序产生异常而终止。

```sh
(gdb) run
Starting program: /home/tunkshif/Documents/Learning/rcore-study/creates/exercises/ch0/src/exception
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/usr/lib/libthread_db.so.1".

Program received signal SIGSEGV, Segmentation fault.
0x0000555555555154 in main (argc=1, argv=0x7fffffffe3f8) at exception.c:6
6         printf("*p = %d", *p);
```

使用 `disassemble` 命令反汇编，查看具体产生异常的一条指令。

```sh
(gdb) disassemble
Dump of assembler code for function main:
   0x0000555555555139 <+0>:     push   %rbp
   0x000055555555513a <+1>:     mov    %rsp,%rbp
   0x000055555555513d <+4>:     sub    $0x20,%rsp
   0x0000555555555141 <+8>:     mov    %edi,-0x14(%rbp)
   0x0000555555555144 <+11>:    mov    %rsi,-0x20(%rbp)
   0x0000555555555148 <+15>:    movq   $0x0,-0x8(%rbp)
   0x0000555555555150 <+23>:    mov    -0x8(%rbp),%rax
=> 0x0000555555555154 <+27>:    mov    (%rax),%eax
   0x0000555555555156 <+29>:    mov    %eax,%esi
   0x0000555555555158 <+31>:    lea    0xea5(%rip),%rax        # 0x555555556004
   0x000055555555515f <+38>:    mov    %rax,%rdi
   0x0000555555555162 <+41>:    mov    $0x0,%eax
   0x0000555555555167 <+46>:    call   0x555555555030 <printf@plt>
   0x000055555555516c <+51>:    mov    $0x0,%eax
   0x0000555555555171 <+56>:    leave
   0x0000555555555172 <+57>:    ret
End of assembler dump.
```

#### 常用命令

- `list`：查看指定函数或指定行
- `run`：运行程序
- `break [location]`：在目标位置设置断点，可以传入行号、函数名或指令地址
- `continue`：执行程序到断点位置处停止
- `stepi`：单步执行一条指令
- `info registers`：查看寄存器内容信息
- `x/i [location]`：查看某地址位置处的指令
- `x/10i [location]`：查看某地址位置处开始的 10 条指令
- `x/10i $pc`：查看接下来将要执行的 10 条指令
- `disassemble`：反汇编查看某段内存中的指令
- `quit`：退出调试

### 使用 strace 工具

`strace` 命令能够用来追踪一个程序进行了哪些系统调用。

例如对于下面一个简单的例子：

```rust
fn main() {
    println!("Hello, world!");
}
```

使用 `cargo build --release` 编译后，用 `strace` 命令执行编译得到的程序，会产生一系列的各种系统调用。而与我们编写的代码相关的系统调用实际只有下面这两条：

```
write(1, "Hello, world!\n", 14Hello, world!)
exit_group(0)
```

### 阅读汇编

下面是一个 GNU Assembler 格式的汇编程序。

```asm
# hello.s
    .global _start

    .text
_start:
    # write(1, message, 13)
    mov     $1, %rax                # system call 1 is write
    mov     $1, %rdi                # file handle 1 is stdout
    mov     $message, %rsi          # address of string to output
    mov     $13, %rdx               # number of bytes
    syscall                         # invoke operating system to do the write

    # exit(0)
    mov     $60, %rax               # system call 60 is exit
    xor     %rdi, %rdi              # we want return code 0
    syscall                         # invoke operating system to exit
message:
    .ascii  "Hello, world\n"
```

其中以 `.directive` 这样的叫做 *directive*，`label:` 叫做 *label*。

`.global` 用于声明导出全局的符号，这样在生成的目标二进制文件（`a.o`）中会存在指向目标代码的符号。
`.text` 用于定义当前的段为代码段，即 `.text` 以下的指令都处于代码段。
`.ascii` 用于生成一个以 `\0` 结尾的字符串。

另外常用的 directive 还有 `.bss` 和 `.data`，分别用于定义 `bss` 段和 `data` 段。

更多资料可以参考下面这些链接的内容：

- [GNU Assembler Examples](https://cs.lmu.edu/~ray/notes/gasexamples/)
- [Oracle x86 Assembly Language Reference Manual - Directives](https://docs.oracle.com/cd/E26502_01/html/E28388/eoiyg.html)
- [x86 Assembly System Call](https://en.wikibooks.org/wiki/X86_Assembly/Interfacing_with_Linux)

[0]: https://rcore-os.github.io/rCore-Tutorial-Book-v3/index.html#
[1]: https://rcore-os.github.io/rCore-Tutorial-deploy/
