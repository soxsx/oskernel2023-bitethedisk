OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

SECTIONS
{
    . = 0x80200000;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        . = ALIGN(0x1000);
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

    sbss = .;
    .bss : {
        skstack0 = .;
        *(.bss.stack)
        ekstack0 = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(0x1000);
    ebss = .;

    ekernel = .;
}
