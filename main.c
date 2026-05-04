#define _GNU_SOURCE

#include "ioctl-wrapper.h"
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <poll.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/inotify.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>
#include <xkbcommon/xkbcommon.h>

#define INPUT_DIR "/dev/input"
#define EVENT_PREFIX "event"
#define SOCKET_PATH "/run/caps-lock-daemon.sock"
#define MAX_DEVICES 128
#define MAX_CLIENTS 64
#define XKB_EVDEV_OFFSET 8

static volatile sig_atomic_t running = 1;

struct input_device {
  int fd;
  char path[256];
  char name[256];
};

struct socket_server {
  int listen_fd;
  int clients[MAX_CLIENTS];
};

static void handle_signal(int signal_number) {
  (void)signal_number;
  running = 0;
}

static int setup_signals(void) {
  if (signal(SIGPIPE, SIG_IGN) == SIG_ERR) {
    perror("signal(SIGPIPE)");
    return -1;
  }

  struct sigaction action = {0};
  action.sa_handler = handle_signal;
  sigemptyset(&action.sa_mask);

  if (sigaction(SIGINT, &action, NULL) < 0) {
    perror("sigaction(SIGINT)");
    return -1;
  }

  if (sigaction(SIGTERM, &action, NULL) < 0) {
    perror("sigaction(SIGTERM)");
    return -1;
  }

  return 0;
}

static void init_socket_server(struct socket_server *server) {
  server->listen_fd = -1;
  for (int i = 0; i < MAX_CLIENTS; i++) {
    server->clients[i] = -1;
  }
}

static bool socket_path_has_listener(void) {
  int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
  if (fd < 0) {
    return true;
  }

  struct sockaddr_un addr = {0};
  addr.sun_family = AF_UNIX;
  snprintf(addr.sun_path, sizeof(addr.sun_path), "%s", SOCKET_PATH);

  bool has_listener = connect(fd, (struct sockaddr *)&addr, sizeof(addr)) == 0;
  close(fd);
  return has_listener;
}

static int bind_socket_path(int fd, struct sockaddr_un *addr) {
  if (bind(fd, (struct sockaddr *)addr, sizeof(*addr)) == 0) {
    return 0;
  }

  if (errno != EADDRINUSE) {
    fprintf(stderr, "bind(%s): %s\n", SOCKET_PATH, strerror(errno));
    return -1;
  }

  if (socket_path_has_listener()) {
    fprintf(stderr, "bind(%s): another daemon appears to be running\n",
            SOCKET_PATH);
    return -1;
  }

  if (unlink(SOCKET_PATH) < 0) {
    fprintf(stderr, "unlink stale %s: %s\n", SOCKET_PATH, strerror(errno));
    return -1;
  }

  if (bind(fd, (struct sockaddr *)addr, sizeof(*addr)) < 0) {
    fprintf(stderr, "bind(%s): %s\n", SOCKET_PATH, strerror(errno));
    return -1;
  }

  return 0;
}

static int setup_socket_server(struct socket_server *server) {
  int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_NONBLOCK | SOCK_CLOEXEC, 0);
  if (fd < 0) {
    perror("socket(AF_UNIX)");
    return -1;
  }

  struct sockaddr_un addr = {0};
  addr.sun_family = AF_UNIX;
  if (snprintf(addr.sun_path, sizeof(addr.sun_path), "%s", SOCKET_PATH) >=
      (int)sizeof(addr.sun_path)) {
    fprintf(stderr, "socket path too long: %s\n", SOCKET_PATH);
    close(fd);
    return -1;
  }

  if (bind_socket_path(fd, &addr) < 0) {
    close(fd);
    return -1;
  }

  if (chmod(SOCKET_PATH, 0666) < 0) {
    fprintf(stderr, "chmod(%s): %s\n", SOCKET_PATH, strerror(errno));
    close(fd);
    unlink(SOCKET_PATH);
    return -1;
  }

  if (listen(fd, SOMAXCONN) < 0) {
    perror("listen");
    close(fd);
    unlink(SOCKET_PATH);
    return -1;
  }

  server->listen_fd = fd;
  fprintf(stderr, "listening on %s\n", SOCKET_PATH);
  return 0;
}

static void close_socket_server(struct socket_server *server) {
  if (server->listen_fd >= 0) {
    close(server->listen_fd);
    server->listen_fd = -1;
  }

  for (int i = 0; i < MAX_CLIENTS; i++) {
    if (server->clients[i] >= 0) {
      close(server->clients[i]);
      server->clients[i] = -1;
    }
  }

  unlink(SOCKET_PATH);
}

static void remove_client(struct socket_server *server, int index) {
  if (server->clients[index] >= 0) {
    close(server->clients[index]);
    server->clients[index] = -1;
  }
}

static int send_caps_state(int fd, bool caps_active) {
  char byte = caps_active ? '1' : '0';
  ssize_t bytes_sent = send(fd, &byte, 1, MSG_NOSIGNAL);
  if (bytes_sent == 1) {
    return 0;
  }

  if (bytes_sent < 0 &&
      (errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR)) {
    return 0;
  }

  return -1;
}

static void broadcast_caps_state(struct socket_server *server,
                                 bool caps_active) {
  for (int i = 0; i < MAX_CLIENTS; i++) {
    if (server->clients[i] >= 0 &&
        send_caps_state(server->clients[i], caps_active) < 0) {
      remove_client(server, i);
    }
  }
}

static int add_client(struct socket_server *server, int fd, bool caps_active) {
  for (int i = 0; i < MAX_CLIENTS; i++) {
    if (server->clients[i] < 0) {
      server->clients[i] = fd;
      if (send_caps_state(fd, caps_active) < 0) {
        remove_client(server, i);
      }
      return 0;
    }
  }

  close(fd);
  fprintf(stderr, "client capacity reached; rejected connection\n");
  return 0;
}

static int accept_clients(struct socket_server *server, bool caps_active) {
  for (;;) {
    int fd =
        accept4(server->listen_fd, NULL, NULL, SOCK_NONBLOCK | SOCK_CLOEXEC);
    if (fd >= 0) {
      add_client(server, fd, caps_active);
      continue;
    }

    if (errno == EAGAIN || errno == EWOULDBLOCK) {
      return 0;
    }

    if (errno == EINTR) {
      return 0;
    }

    perror("accept4");
    return -1;
  }
}

static void reap_disconnected_clients(struct socket_server *server,
                                      struct pollfd pollfds[],
                                      nfds_t first_client_index) {
  for (int i = 0; i < MAX_CLIENTS; i++) {
    if (server->clients[i] < 0) {
      continue;
    }

    short revents = pollfds[first_client_index + (nfds_t)i].revents;
    if ((revents & (POLLHUP | POLLERR | POLLNVAL)) != 0) {
      remove_client(server, i);
    }
  }
}

static bool device_has_caps_lock(int fd) {
  return ioctl_has_EV_KEY_bit(fd) && ioctl_has_KEY_CAPSLOCK_bit(fd);
}

static int open_input_devices(struct input_device devices[], nfds_t capacity) {
  DIR *dir = opendir(INPUT_DIR);
  if (dir == NULL) {
    perror("opendir(" INPUT_DIR ")");
    return -1;
  }

  int count = 0;
  struct dirent *entry;
  while ((entry = readdir(dir)) != NULL) {
    fprintf(stderr, "d_name = %s\n", entry->d_name);
    if (strncmp(entry->d_name, EVENT_PREFIX, strlen(EVENT_PREFIX)) != 0) {
      continue;
    }

    if ((nfds_t)count >= capacity) {
      fprintf(stderr,
              "device capacity reached; ignoring additional input devices\n");
      break;
    }

    char path[256];
    int written =
        snprintf(path, sizeof(path), "%s/%s", INPUT_DIR, entry->d_name);
    if (written < 0 || (size_t)written >= sizeof(path)) {
      continue;
    }

    int fd = open(path, O_RDONLY | O_NONBLOCK | O_CLOEXEC);
    if (fd < 0) {
      fprintf(stderr, "cannot open %s: %s\n", path, strerror(errno));
      continue;
    }

    if (!device_has_caps_lock(fd)) {
      close(fd);
      continue;
    }

    struct input_device *device = &devices[count];
    device->fd = fd;
    snprintf(device->path, sizeof(device->path), "%s", path);

    ioctl_get_DEVICE_NAME(fd, device->name, sizeof(device->name));

    fprintf(stderr, "watching %s (%s)\n", device->path, device->name);
    count++;
  }

  closedir(dir);
  return count;
}

static void close_input_devices(struct input_device devices[], int count) {
  for (int i = 0; i < count; i++) {
    if (devices[i].fd >= 0) {
      close(devices[i].fd);
      devices[i].fd = -1;
    }
  }
}

static int setup_input_dir_watcher(void) {
  int fd = inotify_init1(IN_NONBLOCK | IN_CLOEXEC);
  if (fd < 0) {
    fprintf(stderr, "inotify_init1: %s\n", strerror(errno));
    return -1;
  }

  if (inotify_add_watch(fd, INPUT_DIR,
                        IN_CREATE | IN_DELETE | IN_ATTRIB | IN_MOVED_FROM |
                            IN_MOVED_TO) < 0) {
    fprintf(stderr, "inotify_add_watch(" INPUT_DIR "): %s\n", strerror(errno));
    close(fd);
    return -1;
  }

  return fd;
}

static void drain_input_dir_events(int fd) {
  for (;;) {
    char buffer[4096];
    ssize_t bytes_read = read(fd, buffer, sizeof(buffer));
    if (bytes_read > 0) {
      continue;
    }

    if (bytes_read < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
      return;
    }

    if (bytes_read < 0 && errno == EINTR) {
      return;
    }

    return;
  }
}

static bool any_device_reports_caps_led_active(struct input_device devices[],
                                               int count) {
  for (int i = 0; i < count; i++) {
    if (ioctl_has_CAPSLOCK_LED_bit(devices[i].fd)) {
      return true;
    }
  }

  return false;
}

static struct xkb_keymap *create_keymap(struct xkb_context *context) {
  struct xkb_rule_names names = {
      .rules = getenv("XKB_DEFAULT_RULES"),
      .model = getenv("XKB_DEFAULT_MODEL"),
      .layout = getenv("XKB_DEFAULT_LAYOUT"),
      .variant = getenv("XKB_DEFAULT_VARIANT"),
      .options = getenv("XKB_DEFAULT_OPTIONS"),
  };

  if (names.rules == NULL) {
    names.rules = "evdev";
  }

  if (names.model == NULL) {
    names.model = "pc105";
  }

  if (names.layout == NULL) {
    names.layout = "us";
  }

  return xkb_keymap_new_from_names(context, &names,
                                   XKB_KEYMAP_COMPILE_NO_FLAGS);
}

static void seed_caps_lock_state(struct xkb_keymap *keymap,
                                 struct xkb_state *state, bool caps_active) {
  if (!caps_active) {
    return;
  }

  xkb_mod_index_t caps_index =
      xkb_keymap_mod_get_index(keymap, XKB_MOD_NAME_CAPS);
  if (caps_index == XKB_MOD_INVALID) {
    return;
  }

  xkb_state_update_mask(state, 0, 0, (xkb_mod_mask_t)(1u << caps_index), 0, 0,
                        0);
}

static bool caps_lock_is_active(struct xkb_state *state) {
  return xkb_state_mod_name_is_active(state, XKB_MOD_NAME_CAPS,
                                      XKB_STATE_MODS_LOCKED) > 0;
}

static int process_event(struct xkb_state *state, struct socket_server *server,
                         const struct input_event *event,
                         bool *caps_was_active) {
  if (event->type != EV_KEY || event->code != KEY_CAPSLOCK ||
      event->value == 2) {
    return 0;
  }

  xkb_keycode_t keycode = (xkb_keycode_t)(event->code + XKB_EVDEV_OFFSET);
  enum xkb_key_direction direction =
      event->value == 0 ? XKB_KEY_UP : XKB_KEY_DOWN;
  xkb_state_update_key(state, keycode, direction);

  bool caps_active = caps_lock_is_active(state);
  fprintf(stderr, "%s\n", caps_active ? "activated" : "deactivated");
  if (caps_active != *caps_was_active) {
    broadcast_caps_state(server, caps_active);
  }

  *caps_was_active = caps_active;
  return 0;
}

static int drain_device_events(struct input_device *device,
                               struct xkb_state *state,
                               struct socket_server *server,
                               bool *caps_was_active) {
  for (;;) {
    struct input_event event;
    ssize_t bytes_read = read(device->fd, &event, sizeof(event));

    if (bytes_read == (ssize_t)sizeof(event)) {
      if (process_event(state, server, &event, caps_was_active) < 0) {
        return -1;
      }
      continue;
    }

    if (bytes_read < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
      return 0;
    }

    if (bytes_read < 0 && errno == EINTR) {
      return 0;
    }

    if (bytes_read == 0) {
      fprintf(stderr, "%s reached EOF\n", device->path);
      return -1;
    }

    if (bytes_read < 0) {
      fprintf(stderr, "read(%s): %s\n", device->path, strerror(errno));
    } else {
      fprintf(stderr, "short read from %s\n", device->path);
    }

    return -1;
  }
}

int main(void) {
  if (setup_signals() < 0) {
    return EXIT_FAILURE;
  }

  struct input_device devices[MAX_DEVICES] = {0};
  int device_count = open_input_devices(devices, MAX_DEVICES);
  if (device_count < 0) {
    return EXIT_FAILURE;
  }

  if (device_count == 0) {
    fprintf(stderr, "no readable input devices with KEY_CAPSLOCK found\n");
    return EXIT_FAILURE;
  }

  struct socket_server server;
  init_socket_server(&server);
  if (setup_socket_server(&server) < 0) {
    close_input_devices(devices, device_count);
    return EXIT_FAILURE;
  }

  struct xkb_context *context = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
  if (context == NULL) {
    fprintf(stderr, "failed to create xkb context\n");
    close_socket_server(&server);
    close_input_devices(devices, device_count);
    return EXIT_FAILURE;
  }

  struct xkb_keymap *keymap = create_keymap(context);
  if (keymap == NULL) {
    fprintf(stderr, "failed to create xkb keymap\n");
    xkb_context_unref(context);
    close_socket_server(&server);
    close_input_devices(devices, device_count);
    return EXIT_FAILURE;
  }

  struct xkb_state *state = xkb_state_new(keymap);
  if (state == NULL) {
    fprintf(stderr, "failed to create xkb state\n");
    xkb_keymap_unref(keymap);
    xkb_context_unref(context);
    close_socket_server(&server);
    close_input_devices(devices, device_count);
    return EXIT_FAILURE;
  }

  seed_caps_lock_state(
      keymap, state, any_device_reports_caps_led_active(devices, device_count));
  bool caps_was_active = caps_lock_is_active(state);
  int input_dir_fd = setup_input_dir_watcher();

  while (running) {
    struct pollfd pollfds[2 + MAX_DEVICES + MAX_CLIENTS] = {0};
    nfds_t input_dir_index = 1;
    nfds_t input_index = 2;
    nfds_t client_index = input_index + (nfds_t)device_count;
    nfds_t pollfd_count = client_index + MAX_CLIENTS;

    pollfds[0].fd = server.listen_fd;
    pollfds[0].events = POLLIN;

    pollfds[input_dir_index].fd = input_dir_fd;
    pollfds[input_dir_index].events = POLLIN;

    for (int i = 0; i < device_count; i++) {
      pollfds[input_index + (nfds_t)i].fd = devices[i].fd;
      pollfds[input_index + (nfds_t)i].events = POLLIN;
    }

    for (int i = 0; i < MAX_CLIENTS; i++) {
      pollfds[client_index + (nfds_t)i].fd = server.clients[i];
      pollfds[client_index + (nfds_t)i].events = 0;
    }

    int ready = poll(pollfds, pollfd_count, -1);
    if (ready < 0) {
      if (errno == EINTR) {
        continue;
      }

      perror("poll");
      break;
    }

    if ((pollfds[input_dir_index].revents & POLLIN) != 0) {
      drain_input_dir_events(input_dir_fd);
      fprintf(stderr, INPUT_DIR " changed; rescanning input devices\n");
      close_input_devices(devices, device_count);
      device_count = open_input_devices(devices, MAX_DEVICES);
      if (device_count < 0) {
        break;
      }

      seed_caps_lock_state(
          keymap, state,
          any_device_reports_caps_led_active(devices, device_count));
      caps_was_active = caps_lock_is_active(state);
      continue;
    }

    if ((pollfds[0].revents & POLLIN) != 0) {
      if (accept_clients(&server, caps_was_active) < 0) {
        break;
      }
    }

    for (int i = 0; i < device_count; i++) {
      short revents = pollfds[input_index + (nfds_t)i].revents;
      if ((revents & (POLLHUP | POLLERR | POLLNVAL)) != 0) {
        fprintf(stderr, "%s stopped producing input events (revents=0x%x)\n",
                devices[i].path, revents);
        close(devices[i].fd);
        devices[i].fd = -1;
        continue;
      }

      if ((revents & POLLIN) != 0) {
        if (drain_device_events(&devices[i], state, &server, &caps_was_active) <
            0) {
          close(devices[i].fd);
          devices[i].fd = -1;
        }
      }
    }

    reap_disconnected_clients(&server, pollfds, client_index);
  }

  xkb_state_unref(state);
  xkb_keymap_unref(keymap);
  xkb_context_unref(context);
  if (input_dir_fd >= 0) {
    close(input_dir_fd);
  }
  close_socket_server(&server);
  close_input_devices(devices, device_count);
  return EXIT_SUCCESS;
}
