#pragma once

#include <stdbool.h>

bool ioctl_has_EV_KEY_bit(int fd);
bool ioctl_has_KEY_CAPSLOCK_bit(int fd);
void ioctl_get_DEVICE_NAME(int fd, char *name, unsigned long len);
bool ioctl_has_CAPSLOCK_LED_bit(int fd);
