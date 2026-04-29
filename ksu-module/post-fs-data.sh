# post-fs-data.sh — runs early in boot, pre-zygote.
# Nothing to do at boot for this module; sepolicy.rule is applied
# automatically by KernelSU before init transitions to the system domain.
exit 0
