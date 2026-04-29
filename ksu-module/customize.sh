# KernelSU install hook — runs once at module flash.
SKIPUNZIP=0

ui_print "- APU 650 Unlock v0.3"
ui_print "- target: MediaTek MT6886 (Dimensity 7200 Pro)"
ui_print "- grants shell domain access to /dev/apusys + DMA-buf heaps"

# Sanity check: refuse install on non-MTK hardware.
SOC=$(getprop ro.hardware)
case "$SOC" in
    mt*)  ui_print "- detected MTK SoC: $SOC, proceeding" ;;
    *)    ui_print "! ro.hardware=$SOC is not MTK — aborting"; abort ;;
esac
