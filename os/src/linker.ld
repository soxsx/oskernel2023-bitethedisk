OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

SECTIONS
{
    . = 0x80200000;

    stext = .;
    .text : {
        *(.text.entry)
        . = ALIGN(0x1000); /* 4 KiB 对齐 */
        strampoline = .;
        *(.text.trampoline);
        . = ALIGN(0x1000);
        *(.text .text.*)
    }
    . = ALIGN(0x1000);
    etext = .;

    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(0x1000);
    erodata = .;

    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(0x1000);
    edata = .;

    .bss : {
        skstack = .;
        *(.bss.stack) /* 这里本身就是 8KiB 已经对齐了，所以不需要再手动对齐 */
        ekstack = .;
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(0x1000);
    ebss = .;

    ekernel = .;
}