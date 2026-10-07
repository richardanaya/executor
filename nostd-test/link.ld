/* LM3S6965 (QEMU lm3s6965evb): 256K flash at 0x0, 64K SRAM at 0x20000000. */
MEMORY
{
    FLASH : ORIGIN = 0x00000000, LENGTH = 256K
    RAM   : ORIGIN = 0x20000000, LENGTH = 64K
}

ENTRY(Reset)

SECTIONS
{
    .vector_table ORIGIN(FLASH) :
    {
        LONG(ORIGIN(RAM) + LENGTH(RAM));
        KEEP(*(.vector_table.reset));
        KEEP(*(.vector_table.fault));
    } > FLASH

    .text :
    {
        *(.text .text.*);
        *(.rodata .rodata.*);
    } > FLASH

    .data : ALIGN(4)
    {
        _sdata = .;
        *(.data .data.*);
        _edata = .;
    } > RAM AT > FLASH
    _sidata = LOADADDR(.data);

    .bss : ALIGN(4)
    {
        _sbss = .;
        *(.bss .bss.*);
        *(COMMON);
        _ebss = .;
    } > RAM

    /DISCARD/ :
    {
        *(.ARM.exidx*);
        *(.ARM.extab*);
        *(.eh_frame*);
    }
}
