#include "ioctl-wrapper.h"
#include <linux/input.h>
#include <stdio.h>
#include <sys/ioctl.h>
#include <unistd.h>

#define IOCTL_ALIGNMENT (8 * sizeof(unsigned long))
#define IOCTL_ALIGN(N) ((N + IOCTL_ALIGNMENT) / IOCTL_ALIGNMENT)

static bool test_bit(const unsigned long *bits, unsigned int bit) {
  return (bits[bit / IOCTL_ALIGNMENT] & (1UL << (bit % IOCTL_ALIGNMENT))) != 0;
}

bool ioctl_has_EV_KEY_bit(int fd) {
  unsigned long ev_bits[IOCTL_ALIGN(EV_MAX)] = {0};

  if (ioctl(fd, EVIOCGBIT(0, sizeof(ev_bits)), ev_bits) < 0) {
    return false;
  }

  return test_bit(ev_bits, EV_KEY);
}

bool ioctl_has_KEY_CAPSLOCK_bit(int fd) {
  unsigned long key_bits[IOCTL_ALIGN(KEY_MAX)] = {0};

  if (ioctl(fd, EVIOCGBIT(EV_KEY, sizeof(key_bits)), key_bits) < 0) {
    return false;
  }

  return test_bit(key_bits, KEY_CAPSLOCK);
}

void ioctl_get_DEVICE_NAME(int fd, char *name, unsigned long len) {
  if (ioctl(fd, EVIOCGNAME(len), name) < 0) {
    snprintf(name, len, "unknown");
  }
}

bool ioctl_has_CAPSLOCK_LED_bit(int fd) {
  unsigned long led_bits[IOCTL_ALIGN(LED_MAX)] = {0};

  if (ioctl(fd, EVIOCGLED(sizeof(led_bits)), led_bits) < 0) {
    return false;
  }

  return test_bit(led_bits, LED_CAPSL);
}
