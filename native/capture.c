/*
 * jyoti screen capture.
 *
 * One pthread owns a Wayland connection, captures the selected output with
 * wlr-screencopy v3 and publishes a downscaled RGBA8 grid through a seqlock.
 * The GPU path imports the compositor's linux-dmabuf frame as an EGLImage and
 * box-filters it with a 4x4 tap shader; the wl_shm path strided-samples the
 * compositor's buffer. No colour maths lives here.
 */

#define _GNU_SOURCE

#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <drm_fourcc.h>
#include <errno.h>
#include <fcntl.h>
#include <gbm.h>
#include <poll.h>
#include <pthread.h>
#include <stdarg.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>
#include <wayland-client.h>

#include "linux-dmabuf-protocol.h"
#include "wlr-screencopy-protocol.h"
#include "xdg-output-protocol.h"

#define JYOTI_API __attribute__((visibility("default")))

#define JYOTI_NAME_MAX 64
#define JYOTI_DESCRIPTION_MAX 128
#define JYOTI_MAX_OUTPUTS 16
#define JYOTI_MAX_GRID 4096
#define JYOTI_INIT_TIMEOUT_MS 2000
#define JYOTI_FRAME_TIMEOUT_MS 500

typedef struct {
  char name[JYOTI_NAME_MAX];
  char description[JYOTI_DESCRIPTION_MAX];
  int32_t width;
  int32_t height;
  int32_t refresh_mhz;
} jyoti_monitor;

typedef struct {
  int32_t width;
  int32_t height;
  int64_t age_ms;
} jyoti_frame_info;

/* ------------------------------------------------------------------ */
/* Shared state                                                        */
/* ------------------------------------------------------------------ */

static pthread_mutex_t g_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t g_cond;
static pthread_once_t g_cond_once = PTHREAD_ONCE_INIT;

static pthread_t g_thread;
static _Atomic int g_running;
static _Atomic int g_stop;

static int g_init_state; /* 0 pending, 1 ready, -1 failed */
static int g_error_kind; /* 2 monitor, 3 wayland, 4 capture */
static char g_error[512];

static uint8_t *g_grid;
static int32_t g_grid_width;
static int32_t g_grid_height;
static int32_t g_rate_hz;
static char g_monitor_name[JYOTI_NAME_MAX];

static _Atomic uint32_t g_sequence;
static _Atomic int64_t g_captured_at_ms;

static void init_cond(void) {
  pthread_condattr_t attr;
  pthread_condattr_init(&attr);
  pthread_condattr_setclock(&attr, CLOCK_MONOTONIC);
  pthread_cond_init(&g_cond, &attr);
  pthread_condattr_destroy(&attr);
}

static int64_t now_ns(void) {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (int64_t)ts.tv_sec * 1000000000LL + ts.tv_nsec;
}

static void set_error_kind(int kind, const char *format, ...) {
  va_list args;
  va_start(args, format);
  pthread_mutex_lock(&g_lock);
  g_error_kind = kind;
  vsnprintf(g_error, sizeof g_error, format, args);
  pthread_mutex_unlock(&g_lock);
  va_end(args);
}

static void set_error(const char *format, ...) {
  va_list args;
  va_start(args, format);
  pthread_mutex_lock(&g_lock);
  g_error_kind = 4;
  vsnprintf(g_error, sizeof g_error, format, args);
  pthread_mutex_unlock(&g_lock);
  va_end(args);
}

static void copy_text(char *destination, size_t capacity, const char *source) {
  if (source == NULL) {
    return;
  }
  size_t length = strlen(source);
  if (length >= capacity) {
    length = capacity - 1;
  }
  memcpy(destination, source, length);
  destination[length] = '\0';
}

/* ------------------------------------------------------------------ */
/* Output enumeration                                                  */
/* ------------------------------------------------------------------ */

struct output_entry {
  struct wl_output *output;
  struct zxdg_output_v1 *xdg;
  char name[JYOTI_NAME_MAX];
  char description[JYOTI_DESCRIPTION_MAX];
  int32_t width;
  int32_t height;
  int32_t refresh_mhz;
  int32_t logical_width;
  int32_t logical_height;
  int has_mode;
  int has_logical;
};

struct output_list {
  struct output_entry entries[JYOTI_MAX_OUTPUTS];
  int count;
  struct zxdg_output_manager_v1 *xdg_manager;
};

static void xdg_handle_logical_position(void *data, struct zxdg_output_v1 *xdg, int32_t x, int32_t y) {
  (void)data;
  (void)xdg;
  (void)x;
  (void)y;
}

static void xdg_handle_logical_size(void *data, struct zxdg_output_v1 *xdg, int32_t width, int32_t height) {
  (void)xdg;
  struct output_entry *entry = data;
  entry->logical_width = width;
  entry->logical_height = height;
  entry->has_logical = 1;
}

static void xdg_handle_done(void *data, struct zxdg_output_v1 *xdg) {
  (void)data;
  (void)xdg;
}

static void xdg_handle_name(void *data, struct zxdg_output_v1 *xdg, const char *name) {
  (void)xdg;
  struct output_entry *entry = data;
  copy_text(entry->name, sizeof entry->name, name);
}

static void xdg_handle_description(void *data, struct zxdg_output_v1 *xdg, const char *description) {
  (void)xdg;
  struct output_entry *entry = data;
  copy_text(entry->description, sizeof entry->description, description);
}

static const struct zxdg_output_v1_listener xdg_listener = {
    .logical_position = xdg_handle_logical_position,
    .logical_size = xdg_handle_logical_size,
    .done = xdg_handle_done,
    .name = xdg_handle_name,
    .description = xdg_handle_description,
};

static void output_handle_geometry(void *data, struct wl_output *output, int32_t x, int32_t y,
                                   int32_t physical_width, int32_t physical_height, int32_t subpixel,
                                   const char *make, const char *model, int32_t transform) {
  (void)output;
  (void)x;
  (void)y;
  (void)physical_width;
  (void)physical_height;
  (void)subpixel;
  (void)transform;
  struct output_entry *entry = data;
  if (entry->description[0] == '\0') {
    snprintf(entry->description, sizeof entry->description, "%s %s", make == NULL ? "" : make,
             model == NULL ? "" : model);
  }
}

static void output_handle_mode(void *data, struct wl_output *output, uint32_t flags, int32_t width,
                               int32_t height, int32_t refresh) {
  (void)output;
  struct output_entry *entry = data;
  if ((flags & WL_OUTPUT_MODE_CURRENT) == 0) {
    return;
  }
  entry->width = width;
  entry->height = height;
  entry->refresh_mhz = refresh;
  entry->has_mode = 1;
}

static void output_handle_done(void *data, struct wl_output *output) {
  (void)data;
  (void)output;
}

static void output_handle_scale(void *data, struct wl_output *output, int32_t factor) {
  (void)data;
  (void)output;
  (void)factor;
}

static void output_handle_name(void *data, struct wl_output *output, const char *name) {
  (void)output;
  struct output_entry *entry = data;
  copy_text(entry->name, sizeof entry->name, name);
}

static void output_handle_description(void *data, struct wl_output *output, const char *description) {
  (void)output;
  struct output_entry *entry = data;
  copy_text(entry->description, sizeof entry->description, description);
}

static const struct wl_output_listener output_listener = {
    .geometry = output_handle_geometry,
    .mode = output_handle_mode,
    .done = output_handle_done,
    .scale = output_handle_scale,
    .name = output_handle_name,
    .description = output_handle_description,
};

static void attach_xdg_output(struct output_list *list, struct output_entry *entry) {
  if (list->xdg_manager == NULL || entry->xdg != NULL) {
    return;
  }
  entry->xdg = zxdg_output_manager_v1_get_xdg_output(list->xdg_manager, entry->output);
  if (entry->xdg != NULL) {
    zxdg_output_v1_add_listener(entry->xdg, &xdg_listener, entry);
  }
}

static void registry_global(void *data, struct wl_registry *registry, uint32_t name,
                            const char *interface, uint32_t version) {
  struct output_list *list = data;
  if (strcmp(interface, wl_output_interface.name) == 0 && list->count < JYOTI_MAX_OUTPUTS) {
    uint32_t bound = version < 4 ? version : 4;
    struct wl_output *output = wl_registry_bind(registry, name, &wl_output_interface, bound);
    if (output == NULL) {
      return;
    }
    struct output_entry *entry = &list->entries[list->count];
    memset(entry, 0, sizeof *entry);
    entry->output = output;
    wl_output_add_listener(output, &output_listener, entry);
    list->count += 1;
    attach_xdg_output(list, entry);
    return;
  }
  if (strcmp(interface, zxdg_output_manager_v1_interface.name) == 0 && list->xdg_manager == NULL) {
    uint32_t bound = version < 3 ? version : 3;
    list->xdg_manager =
        wl_registry_bind(registry, name, &zxdg_output_manager_v1_interface, bound);
    for (int index = 0; index < list->count; index += 1) {
      attach_xdg_output(list, &list->entries[index]);
    }
  }
}

static void registry_global_remove(void *data, struct wl_registry *registry, uint32_t name) {
  (void)data;
  (void)registry;
  (void)name;
}

static const struct wl_registry_listener registry_listener = {
    .global = registry_global,
    .global_remove = registry_global_remove,
};

static void free_outputs(struct output_list *list) {
  for (int index = 0; index < list->count; index += 1) {
    struct output_entry *entry = &list->entries[index];
    if (entry->xdg != NULL) {
      zxdg_output_v1_destroy(entry->xdg);
    }
    if (entry->output != NULL) {
      wl_output_destroy(entry->output);
    }
  }
  if (list->xdg_manager != NULL) {
    zxdg_output_manager_v1_destroy(list->xdg_manager);
  }
  list->count = 0;
  list->xdg_manager = NULL;
}

static int collect_outputs(struct wl_display *display, struct output_list *list) {
  struct wl_registry *registry = wl_display_get_registry(display);
  if (registry == NULL) {
    return -1;
  }
  wl_registry_add_listener(registry, &registry_listener, list);
  if (wl_display_roundtrip(display) < 0) {
    wl_registry_destroy(registry);
    return -1;
  }
  if (wl_display_roundtrip(display) < 0) {
    wl_registry_destroy(registry);
    return -1;
  }
  wl_registry_destroy(registry);
  return 0;
}

static void monitor_from_entry(const struct output_entry *entry, jyoti_monitor *monitor) {
  memset(monitor, 0, sizeof *monitor);
  copy_text(monitor->name, sizeof monitor->name, entry->name);
  copy_text(monitor->description, sizeof monitor->description, entry->description);
  if (entry->has_logical) {
    monitor->width = entry->logical_width;
    monitor->height = entry->logical_height;
  } else {
    monitor->width = entry->width;
    monitor->height = entry->height;
  }
  monitor->refresh_mhz = entry->refresh_mhz > 0 ? entry->refresh_mhz : 60000;
}

/* ------------------------------------------------------------------ */
/* GPU downscale                                                       */
/* ------------------------------------------------------------------ */

typedef unsigned int gl_uint;
typedef int gl_int;
typedef int gl_sizei;
typedef unsigned int gl_enum;
typedef float gl_float;
typedef char gl_char;

#define GL_TEXTURE_2D 0x0DE1
#define GL_TEXTURE0 0x84C0
#define GL_TEXTURE_MIN_FILTER 0x2801
#define GL_TEXTURE_MAG_FILTER 0x2800
#define GL_TEXTURE_WRAP_S 0x2802
#define GL_TEXTURE_WRAP_T 0x2803
#define GL_LINEAR 0x2601
#define GL_CLAMP_TO_EDGE 0x812F
#define GL_RGBA 0x1908
#define GL_UNSIGNED_BYTE 0x1401
#define GL_FLOAT 0x1406
#define GL_FRAMEBUFFER 0x8D40
#define GL_COLOR_ATTACHMENT0 0x8CE0
#define GL_FRAMEBUFFER_COMPLETE 0x8CD5
#define GL_ARRAY_BUFFER 0x8892
#define GL_STATIC_DRAW 0x88E4
#define GL_TRIANGLE_STRIP 0x0005
#define GL_VERTEX_SHADER 0x8B31
#define GL_FRAGMENT_SHADER 0x8B30
#define GL_COMPILE_STATUS 0x8B81
#define GL_LINK_STATUS 0x8B82
#define GL_COLOR_BUFFER_BIT 0x4000
#define GL_NO_ERROR 0

typedef void (*pfn_viewport)(gl_int, gl_int, gl_sizei, gl_sizei);
typedef void (*pfn_clear_color)(gl_float, gl_float, gl_float, gl_float);
typedef void (*pfn_clear)(gl_enum);
typedef void (*pfn_gen_textures)(gl_sizei, gl_uint *);
typedef void (*pfn_bind_texture)(gl_enum, gl_uint);
typedef void (*pfn_tex_parameteri)(gl_enum, gl_enum, gl_int);
typedef void (*pfn_tex_image_2d)(gl_enum, gl_int, gl_int, gl_sizei, gl_sizei, gl_int, gl_enum,
                                 gl_enum, const void *);
typedef void (*pfn_gen_framebuffers)(gl_sizei, gl_uint *);
typedef void (*pfn_bind_framebuffer)(gl_enum, gl_uint);
typedef void (*pfn_framebuffer_texture_2d)(gl_enum, gl_enum, gl_enum, gl_uint, gl_int);
typedef gl_enum (*pfn_check_framebuffer_status)(gl_enum);
typedef void (*pfn_read_pixels)(gl_int, gl_int, gl_sizei, gl_sizei, gl_enum, gl_enum, void *);
typedef void (*pfn_gen_buffers)(gl_sizei, gl_uint *);
typedef void (*pfn_bind_buffer)(gl_enum, gl_uint);
typedef void (*pfn_buffer_data)(gl_enum, gl_sizei, const void *, gl_enum);
typedef void (*pfn_enable_vertex_attrib_array)(gl_uint);
typedef void (*pfn_vertex_attrib_pointer)(gl_uint, gl_int, gl_enum, unsigned char, gl_sizei,
                                          const void *);
typedef void (*pfn_draw_arrays)(gl_enum, gl_int, gl_sizei);
typedef gl_uint (*pfn_create_shader)(gl_enum);
typedef void (*pfn_shader_source)(gl_uint, gl_sizei, const gl_char *const *, const gl_int *);
typedef void (*pfn_compile_shader)(gl_uint);
typedef void (*pfn_get_shader_iv)(gl_uint, gl_enum, gl_int *);
typedef gl_uint (*pfn_create_program)(void);
typedef void (*pfn_attach_shader)(gl_uint, gl_uint);
typedef void (*pfn_link_program)(gl_uint);
typedef void (*pfn_get_program_iv)(gl_uint, gl_enum, gl_int *);
typedef void (*pfn_use_program)(gl_uint);
typedef gl_int (*pfn_get_attrib_location)(gl_uint, const gl_char *);
typedef gl_int (*pfn_get_uniform_location)(gl_uint, const gl_char *);
typedef void (*pfn_uniform_1i)(gl_int, gl_int);
typedef void (*pfn_uniform_1f)(gl_int, gl_float);
typedef void (*pfn_uniform_2f)(gl_int, gl_float, gl_float);
typedef void (*pfn_active_texture)(gl_enum);
typedef void (*pfn_delete_textures)(gl_sizei, const gl_uint *);
typedef void (*pfn_delete_framebuffers)(gl_sizei, const gl_uint *);
typedef void (*pfn_delete_shader)(gl_uint);
typedef void (*pfn_delete_program)(gl_uint);
typedef void (*pfn_egl_image_target_texture)(gl_enum, void *);

struct gl_api {
  pfn_viewport viewport;
  pfn_clear_color clear_color;
  pfn_clear clear;
  pfn_gen_textures gen_textures;
  pfn_bind_texture bind_texture;
  pfn_tex_parameteri tex_parameteri;
  pfn_tex_image_2d tex_image_2d;
  pfn_gen_framebuffers gen_framebuffers;
  pfn_bind_framebuffer bind_framebuffer;
  pfn_framebuffer_texture_2d framebuffer_texture_2d;
  pfn_check_framebuffer_status check_framebuffer_status;
  pfn_read_pixels read_pixels;
  pfn_gen_buffers gen_buffers;
  pfn_bind_buffer bind_buffer;
  pfn_buffer_data buffer_data;
  pfn_enable_vertex_attrib_array enable_vertex_attrib_array;
  pfn_vertex_attrib_pointer vertex_attrib_pointer;
  pfn_draw_arrays draw_arrays;
  pfn_create_shader create_shader;
  pfn_shader_source shader_source;
  pfn_compile_shader compile_shader;
  pfn_get_shader_iv get_shader_iv;
  pfn_create_program create_program;
  pfn_attach_shader attach_shader;
  pfn_link_program link_program;
  pfn_get_program_iv get_program_iv;
  pfn_use_program use_program;
  pfn_get_attrib_location get_attrib_location;
  pfn_get_uniform_location get_uniform_location;
  pfn_uniform_1i uniform_1i;
  pfn_uniform_1f uniform_1f;
  pfn_uniform_2f uniform_2f;
  pfn_active_texture active_texture;
  pfn_delete_textures delete_textures;
  pfn_delete_framebuffers delete_framebuffers;
  pfn_delete_shader delete_shader;
  pfn_delete_program delete_program;
  pfn_egl_image_target_texture image_target_texture;
};

struct gpu {
  int ready;
  int egl_initialized;
  int drm_fd;
  struct gbm_device *gbm;
  EGLDisplay display;
  EGLContext context;
  PFNEGLCREATEIMAGEKHRPROC create_image;
  PFNEGLDESTROYIMAGEKHRPROC destroy_image;
  int have_modifiers;
  struct gl_api gl;
  gl_uint source_texture;
  gl_uint target_texture;
  gl_uint framebuffer;
  gl_uint program;
  gl_uint vertex_buffer;
  gl_int uniform_tex;
  gl_int uniform_footprint;
  gl_int attrib_position;
  gl_int attrib_uv;
};

static const char *const VERTEX_SOURCE =
    "attribute vec2 a_position;\n"
    "attribute vec2 a_uv;\n"
    "varying vec2 v_uv;\n"
    "void main() {\n"
    "  v_uv = a_uv;\n"
    "  gl_Position = vec4(a_position, 0.0, 1.0);\n"
    "}\n";

static const char *const FRAGMENT_SOURCE =
    "precision mediump float;\n"
    "varying vec2 v_uv;\n"
    "uniform sampler2D u_texture;\n"
    "uniform vec2 u_footprint;\n"
    "void main() {\n"
    "  vec3 sum = vec3(0.0);\n"
    "  for (int y = 0; y < 4; y++) {\n"
    "    for (int x = 0; x < 4; x++) {\n"
    "      vec2 offset = (vec2(float(x), float(y)) + 0.5) * 0.25 - 0.5;\n"
    "      sum += texture2D(u_texture, v_uv + offset * u_footprint).rgb;\n"
    "    }\n"
    "  }\n"
    "  gl_FragColor = vec4(sum * 0.0625, 1.0);\n"
    "}\n";

#define LOAD_GL(field, type, name)                                     \
  do {                                                                 \
    api->field = (type)eglGetProcAddress(name);                         \
    if (api->field == NULL) {                                          \
      return -1;                                                       \
    }                                                                  \
  } while (0)

static int load_gl(struct gl_api *api) {
  LOAD_GL(viewport, pfn_viewport, "glViewport");
  LOAD_GL(clear_color, pfn_clear_color, "glClearColor");
  LOAD_GL(clear, pfn_clear, "glClear");
  LOAD_GL(gen_textures, pfn_gen_textures, "glGenTextures");
  LOAD_GL(bind_texture, pfn_bind_texture, "glBindTexture");
  LOAD_GL(tex_parameteri, pfn_tex_parameteri, "glTexParameteri");
  LOAD_GL(tex_image_2d, pfn_tex_image_2d, "glTexImage2D");
  LOAD_GL(gen_framebuffers, pfn_gen_framebuffers, "glGenFramebuffers");
  LOAD_GL(bind_framebuffer, pfn_bind_framebuffer, "glBindFramebuffer");
  LOAD_GL(framebuffer_texture_2d, pfn_framebuffer_texture_2d, "glFramebufferTexture2D");
  LOAD_GL(check_framebuffer_status, pfn_check_framebuffer_status, "glCheckFramebufferStatus");
  LOAD_GL(read_pixels, pfn_read_pixels, "glReadPixels");
  LOAD_GL(gen_buffers, pfn_gen_buffers, "glGenBuffers");
  LOAD_GL(bind_buffer, pfn_bind_buffer, "glBindBuffer");
  LOAD_GL(buffer_data, pfn_buffer_data, "glBufferData");
  LOAD_GL(enable_vertex_attrib_array, pfn_enable_vertex_attrib_array,
          "glEnableVertexAttribArray");
  LOAD_GL(vertex_attrib_pointer, pfn_vertex_attrib_pointer, "glVertexAttribPointer");
  LOAD_GL(draw_arrays, pfn_draw_arrays, "glDrawArrays");
  LOAD_GL(create_shader, pfn_create_shader, "glCreateShader");
  LOAD_GL(shader_source, pfn_shader_source, "glShaderSource");
  LOAD_GL(compile_shader, pfn_compile_shader, "glCompileShader");
  LOAD_GL(get_shader_iv, pfn_get_shader_iv, "glGetShaderiv");
  LOAD_GL(create_program, pfn_create_program, "glCreateProgram");
  LOAD_GL(attach_shader, pfn_attach_shader, "glAttachShader");
  LOAD_GL(link_program, pfn_link_program, "glLinkProgram");
  LOAD_GL(get_program_iv, pfn_get_program_iv, "glGetProgramiv");
  LOAD_GL(use_program, pfn_use_program, "glUseProgram");
  LOAD_GL(get_attrib_location, pfn_get_attrib_location, "glGetAttribLocation");
  LOAD_GL(get_uniform_location, pfn_get_uniform_location, "glGetUniformLocation");
  LOAD_GL(uniform_1i, pfn_uniform_1i, "glUniform1i");
  LOAD_GL(uniform_1f, pfn_uniform_1f, "glUniform1f");
  LOAD_GL(uniform_2f, pfn_uniform_2f, "glUniform2f");
  LOAD_GL(active_texture, pfn_active_texture, "glActiveTexture");
  LOAD_GL(delete_textures, pfn_delete_textures, "glDeleteTextures");
  LOAD_GL(delete_framebuffers, pfn_delete_framebuffers, "glDeleteFramebuffers");
  LOAD_GL(delete_shader, pfn_delete_shader, "glDeleteShader");
  LOAD_GL(delete_program, pfn_delete_program, "glDeleteProgram");
  LOAD_GL(image_target_texture, pfn_egl_image_target_texture, "glEGLImageTargetTexture2DOES");
  return 0;
}

static gl_uint compile_shader(struct gl_api *gl, gl_enum type, const char *source) {
  gl_uint shader = gl->create_shader(type);
  if (shader == 0) {
    return 0;
  }
  gl->shader_source(shader, 1, &source, NULL);
  gl->compile_shader(shader);
  gl_int compiled = 0;
  gl->get_shader_iv(shader, GL_COMPILE_STATUS, &compiled);
  if (compiled == 0) {
    gl->delete_shader(shader);
    return 0;
  }
  return shader;
}

static int open_render_node(void) {
  for (int index = 128; index < 136; index += 1) {
    char path[64];
    snprintf(path, sizeof path, "/dev/dri/renderD%d", index);
    int fd = open(path, O_RDWR | O_CLOEXEC);
    if (fd >= 0) {
      return fd;
    }
  }
  return -1;
}

static int gpu_init(struct gpu *gpu) {
  memset(gpu, 0, sizeof *gpu);
  gpu->drm_fd = -1;
  gpu->display = EGL_NO_DISPLAY;

  PFNEGLGETPLATFORMDISPLAYEXTPROC get_platform_display =
      (PFNEGLGETPLATFORMDISPLAYEXTPROC)eglGetProcAddress("eglGetPlatformDisplayEXT");
  if (get_platform_display == NULL) {
    return -1;
  }

  gpu->drm_fd = open_render_node();
  if (gpu->drm_fd < 0) {
    return -1;
  }
  gpu->gbm = gbm_create_device(gpu->drm_fd);
  if (gpu->gbm == NULL) {
    return -1;
  }

  gpu->display = get_platform_display(EGL_PLATFORM_GBM_KHR, gpu->gbm, NULL);
  if (gpu->display == EGL_NO_DISPLAY) {
    return -1;
  }
  EGLint major = 0;
  EGLint minor = 0;
  if (eglInitialize(gpu->display, &major, &minor) == EGL_FALSE) {
    return -1;
  }
  gpu->egl_initialized = 1;
  if (eglBindAPI(EGL_OPENGL_ES_API) == EGL_FALSE) {
    return -1;
  }

  const char *extensions = eglQueryString(gpu->display, EGL_EXTENSIONS);
  if (extensions == NULL) {
    return -1;
  }
  gpu->have_modifiers = strstr(extensions, "EGL_EXT_image_dma_buf_import_modifiers") != NULL;
  if (strstr(extensions, "EGL_EXT_image_dma_buf_import") == NULL ||
      strstr(extensions, "EGL_KHR_image_base") == NULL) {
    return -1;
  }

  const EGLint config_attributes[] = {
      EGL_SURFACE_TYPE, EGL_WINDOW_BIT,
      EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
      EGL_NONE,
  };
  EGLConfig config;
  EGLint config_count = 0;
  if (eglChooseConfig(gpu->display, config_attributes, &config, 1, &config_count) == EGL_FALSE ||
      config_count == 0) {
    return -1;
  }

  const EGLint context_attributes[] = {
      EGL_CONTEXT_CLIENT_VERSION, 2,
      EGL_NONE,
  };
  gpu->context = eglCreateContext(gpu->display, config, EGL_NO_CONTEXT, context_attributes);
  if (gpu->context == EGL_NO_CONTEXT) {
    return -1;
  }
  if (eglMakeCurrent(gpu->display, EGL_NO_SURFACE, EGL_NO_SURFACE, gpu->context) == EGL_FALSE) {
    return -1;
  }

  gpu->create_image = (PFNEGLCREATEIMAGEKHRPROC)eglGetProcAddress("eglCreateImageKHR");
  gpu->destroy_image = (PFNEGLDESTROYIMAGEKHRPROC)eglGetProcAddress("eglDestroyImageKHR");
  if (gpu->create_image == NULL || gpu->destroy_image == NULL) {
    return -1;
  }
  if (load_gl(&gpu->gl) != 0) {
    return -1;
  }

  struct gl_api *gl = &gpu->gl;
  gl_uint vertex_shader = compile_shader(gl, GL_VERTEX_SHADER, VERTEX_SOURCE);
  gl_uint fragment_shader = compile_shader(gl, GL_FRAGMENT_SHADER, FRAGMENT_SOURCE);
  if (vertex_shader == 0 || fragment_shader == 0) {
    return -1;
  }
  gpu->program = gl->create_program();
  if (gpu->program == 0) {
    return -1;
  }
  gl->attach_shader(gpu->program, vertex_shader);
  gl->attach_shader(gpu->program, fragment_shader);
  gl->link_program(gpu->program);
  gl_int linked = 0;
  gl->get_program_iv(gpu->program, GL_LINK_STATUS, &linked);
  gl->delete_shader(vertex_shader);
  gl->delete_shader(fragment_shader);
  if (linked == 0) {
    return -1;
  }

  gpu->attrib_position = gl->get_attrib_location(gpu->program, "a_position");
  gpu->attrib_uv = gl->get_attrib_location(gpu->program, "a_uv");
  gpu->uniform_tex = gl->get_uniform_location(gpu->program, "u_texture");
  gpu->uniform_footprint = gl->get_uniform_location(gpu->program, "u_footprint");
  if (gpu->attrib_position < 0 || gpu->attrib_uv < 0 || gpu->uniform_tex < 0 ||
      gpu->uniform_footprint < 0) {
    return -1;
  }

  const float vertices[] = {
      -1.0f, -1.0f, 0.0f, 0.0f,
      1.0f,  -1.0f, 1.0f, 0.0f,
      -1.0f, 1.0f,  0.0f, 1.0f,
      1.0f,  1.0f,  1.0f, 1.0f,
  };
  gl->gen_buffers(1, &gpu->vertex_buffer);
  gl->bind_buffer(GL_ARRAY_BUFFER, gpu->vertex_buffer);
  gl->buffer_data(GL_ARRAY_BUFFER, (gl_sizei)sizeof vertices, vertices, GL_STATIC_DRAW);

  gl->gen_textures(1, &gpu->source_texture);
  gl->bind_texture(GL_TEXTURE_2D, gpu->source_texture);
  gl->tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
  gl->tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
  gl->tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
  gl->tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);

  gpu->ready = 1;
  return 0;
}

static void gpu_finish(struct gpu *gpu) {
  if (gpu->egl_initialized) {
    if (gpu->ready) {
      gpu->gl.delete_framebuffers(1, &gpu->framebuffer);
      gpu->gl.delete_textures(1, &gpu->target_texture);
      gpu->gl.delete_textures(1, &gpu->source_texture);
      gpu->gl.delete_program(gpu->program);
    }
    eglMakeCurrent(gpu->display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
    if (gpu->context != EGL_NO_CONTEXT) {
      eglDestroyContext(gpu->display, gpu->context);
    }
    eglTerminate(gpu->display);
  }
  if (gpu->gbm != NULL) {
    gbm_device_destroy(gpu->gbm);
  }
  if (gpu->drm_fd >= 0) {
    close(gpu->drm_fd);
  }
  memset(gpu, 0, sizeof *gpu);
  gpu->drm_fd = -1;
  gpu->display = EGL_NO_DISPLAY;
}

static int gpu_resize_target(struct gpu *gpu, int width, int height) {
  if (gpu->framebuffer != 0) {
    return 0;
  }
  struct gl_api *gl = &gpu->gl;
  gl->gen_textures(1, &gpu->target_texture);
  gl->bind_texture(GL_TEXTURE_2D, gpu->target_texture);
  gl->tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
  gl->tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
  gl->tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
  gl->tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
  gl->tex_image_2d(GL_TEXTURE_2D, 0, GL_RGBA, width, height, 0, GL_RGBA, GL_UNSIGNED_BYTE, NULL);
  gl->gen_framebuffers(1, &gpu->framebuffer);
  gl->bind_framebuffer(GL_FRAMEBUFFER, gpu->framebuffer);
  gl->framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D,
                             gpu->target_texture, 0);
  if (gl->check_framebuffer_status(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
    return -1;
  }
  return 0;
}

static EGLImage gpu_image_from_bo(struct gpu *gpu, struct gbm_bo *bo, uint32_t format,
                                  int width, int height) {
  int fd = gbm_bo_get_fd(bo);
  if (fd < 0) {
    return EGL_NO_IMAGE;
  }
  uint64_t modifier = gbm_bo_get_modifier(bo);
  EGLint attributes[20];
  int cursor = 0;
  attributes[cursor++] = EGL_WIDTH;
  attributes[cursor++] = width;
  attributes[cursor++] = EGL_HEIGHT;
  attributes[cursor++] = height;
  attributes[cursor++] = EGL_LINUX_DRM_FOURCC_EXT;
  attributes[cursor++] = (EGLint)format;
  attributes[cursor++] = EGL_DMA_BUF_PLANE0_FD_EXT;
  attributes[cursor++] = fd;
  attributes[cursor++] = EGL_DMA_BUF_PLANE0_OFFSET_EXT;
  attributes[cursor++] = (EGLint)gbm_bo_get_offset(bo, 0);
  attributes[cursor++] = EGL_DMA_BUF_PLANE0_PITCH_EXT;
  attributes[cursor++] = (EGLint)gbm_bo_get_stride(bo);
  if (gpu->have_modifiers) {
    attributes[cursor++] = EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT;
    attributes[cursor++] = (EGLint)(modifier & 0xffffffffu);
    attributes[cursor++] = EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT;
    attributes[cursor++] = (EGLint)(modifier >> 32);
  }
  attributes[cursor++] = EGL_NONE;

  EGLImage image =
      gpu->create_image(gpu->display, EGL_NO_CONTEXT, EGL_LINUX_DMA_BUF_EXT, NULL, attributes);
  close(fd);
  if (image == EGL_NO_IMAGE) {
    return EGL_NO_IMAGE;
  }
  gpu->gl.bind_texture(GL_TEXTURE_2D, gpu->source_texture);
  gpu->gl.image_target_texture(GL_TEXTURE_2D, image);
  return image;
}

static int gpu_render(struct gpu *gpu, int grid_width, int grid_height, int y_invert,
                      uint8_t *output) {
  if (!gpu->ready || gpu_resize_target(gpu, grid_width, grid_height) != 0) {
    return -1;
  }
  struct gl_api *gl = &gpu->gl;
  gl->bind_framebuffer(GL_FRAMEBUFFER, gpu->framebuffer);
  gl->viewport(0, 0, grid_width, grid_height);
  gl->clear_color(0.0f, 0.0f, 0.0f, 1.0f);
  gl->clear(GL_COLOR_BUFFER_BIT);
  gl->use_program(gpu->program);
  gl->active_texture(GL_TEXTURE0);
  gl->bind_texture(GL_TEXTURE_2D, gpu->source_texture);
  gl->uniform_1i(gpu->uniform_tex, 0);
  gl->uniform_2f(gpu->uniform_footprint, 1.0f / (float)grid_width, 1.0f / (float)grid_height);
  gl->bind_buffer(GL_ARRAY_BUFFER, gpu->vertex_buffer);
  gl->enable_vertex_attrib_array((gl_uint)gpu->attrib_position);
  gl->enable_vertex_attrib_array((gl_uint)gpu->attrib_uv);
  gl->vertex_attrib_pointer((gl_uint)gpu->attrib_position, 2, GL_FLOAT, 0, 4 * sizeof(float),
                            (const void *)0);
  gl->vertex_attrib_pointer((gl_uint)gpu->attrib_uv, 2, GL_FLOAT, 0, 4 * sizeof(float),
                            (const void *)(2 * sizeof(float)));
  gl->draw_arrays(GL_TRIANGLE_STRIP, 0, 4);
  gl->read_pixels(0, 0, grid_width, grid_height, GL_RGBA, GL_UNSIGNED_BYTE, output);

  /*
   * glReadPixels is bottom-up. Without y_invert the texture's first row is the
   * image top, so the first row read is already the image top; with y_invert
   * the buffer is bottom-up and the rows need reversing.
   */
  if (y_invert) {
    size_t row = (size_t)grid_width * 4;
    uint8_t *scratch = malloc(row);
    if (scratch == NULL) {
      return -1;
    }
    for (int y = 0; y < grid_height / 2; y += 1) {
      uint8_t *top = output + (size_t)y * row;
      uint8_t *bottom = output + (size_t)(grid_height - 1 - y) * row;
      memcpy(scratch, top, row);
      memcpy(top, bottom, row);
      memcpy(bottom, scratch, row);
    }
    free(scratch);
  }
  return 0;
}

/* ------------------------------------------------------------------ */
/* Capture                                                             */
/* ------------------------------------------------------------------ */

struct capture {
  struct wl_display *display;
  struct wl_registry *registry;
  struct wl_output *output;
  struct zwlr_screencopy_manager_v1 *manager;
  struct wl_shm *shm;
  struct zwp_linux_dmabuf_v1 *dmabuf;
  struct output_list outputs;
  struct gpu gpu;
  int use_dmabuf;
  int gpu_attempted;
  struct wl_buffer *frame_buffer;
  struct gbm_bo *frame_bo;
  EGLImage frame_image;
  void *shm_data;
  size_t shm_size;
  uint32_t frame_format;
  int32_t frame_width;
  int32_t frame_height;
  int32_t frame_stride;
  int frame_is_dmabuf;
};

struct frame {
  struct capture *capture;
  struct zwlr_screencopy_frame_v1 *proxy;
  int ready;
  int failed;
  int copied;
  int y_invert;
  uint32_t shm_format;
  uint32_t shm_width;
  uint32_t shm_height;
  uint32_t shm_stride;
  int have_shm;
  uint32_t dmabuf_format;
  uint32_t dmabuf_width;
  uint32_t dmabuf_height;
  int have_dmabuf;
};

static void frame_handle_buffer(void *data, struct zwlr_screencopy_frame_v1 *proxy,
                                uint32_t format, uint32_t width, uint32_t height,
                                uint32_t stride) {
  (void)proxy;
  struct frame *frame = data;
  frame->shm_format = format;
  frame->shm_width = width;
  frame->shm_height = height;
  frame->shm_stride = stride;
  frame->have_shm = 1;
}

static void frame_handle_flags(void *data, struct zwlr_screencopy_frame_v1 *proxy, uint32_t flags) {
  (void)proxy;
  struct frame *frame = data;
  frame->y_invert = (flags & ZWLR_SCREENCOPY_FRAME_V1_FLAGS_Y_INVERT) != 0;
}

static void frame_handle_ready(void *data, struct zwlr_screencopy_frame_v1 *proxy, uint32_t sec_hi,
                               uint32_t sec_lo, uint32_t nsec) {
  (void)proxy;
  (void)sec_hi;
  (void)sec_lo;
  (void)nsec;
  struct frame *frame = data;
  frame->ready = 1;
}

static void frame_handle_failed(void *data, struct zwlr_screencopy_frame_v1 *proxy) {
  (void)proxy;
  struct frame *frame = data;
  frame->failed = 1;
}

static void frame_handle_damage(void *data, struct zwlr_screencopy_frame_v1 *proxy, uint32_t x,
                                uint32_t y, uint32_t width, uint32_t height) {
  (void)data;
  (void)proxy;
  (void)x;
  (void)y;
  (void)width;
  (void)height;
}

static void frame_handle_linux_dmabuf(void *data, struct zwlr_screencopy_frame_v1 *proxy,
                                      uint32_t format, uint32_t width, uint32_t height) {
  (void)proxy;
  struct frame *frame = data;
  frame->dmabuf_format = format;
  frame->dmabuf_width = width;
  frame->dmabuf_height = height;
  frame->have_dmabuf = 1;
}

static void destroy_frame_buffer(struct capture *capture) {
  if (capture->frame_image != EGL_NO_IMAGE) {
    if (capture->gpu.destroy_image != NULL && capture->gpu.display != EGL_NO_DISPLAY) {
      capture->gpu.destroy_image(capture->gpu.display, capture->frame_image);
    }
    capture->frame_image = EGL_NO_IMAGE;
  }
  if (capture->frame_buffer != NULL) {
    wl_buffer_destroy(capture->frame_buffer);
    capture->frame_buffer = NULL;
  }
  if (capture->frame_bo != NULL) {
    gbm_bo_destroy(capture->frame_bo);
    capture->frame_bo = NULL;
  }
  if (capture->shm_data != NULL) {
    munmap(capture->shm_data, capture->shm_size);
    capture->shm_data = NULL;
    capture->shm_size = 0;
  }
  capture->frame_is_dmabuf = 0;
  capture->frame_format = 0;
  capture->frame_width = 0;
  capture->frame_height = 0;
  capture->frame_stride = 0;
}

static int ensure_shm_buffer(struct capture *capture, struct frame *frame) {
  if (!frame->have_shm) {
    return -1;
  }
  if (capture->frame_buffer != NULL && !capture->frame_is_dmabuf &&
      capture->frame_format == frame->shm_format &&
      capture->frame_width == (int32_t)frame->shm_width &&
      capture->frame_height == (int32_t)frame->shm_height &&
      capture->frame_stride == (int32_t)frame->shm_stride) {
    return 0;
  }
  destroy_frame_buffer(capture);

  size_t size = (size_t)frame->shm_stride * frame->shm_height;
  int fd = memfd_create("jyoti-capture", MFD_CLOEXEC);
  if (fd < 0) {
    return -1;
  }
  if (ftruncate(fd, (off_t)size) < 0) {
    close(fd);
    return -1;
  }
  void *data = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
  if (data == MAP_FAILED) {
    close(fd);
    return -1;
  }
  struct wl_shm_pool *pool = wl_shm_create_pool(capture->shm, fd, (int32_t)size);
  if (pool == NULL) {
    munmap(data, size);
    close(fd);
    return -1;
  }
  struct wl_buffer *buffer = wl_shm_pool_create_buffer(
      pool, 0, (int32_t)frame->shm_width, (int32_t)frame->shm_height, (int32_t)frame->shm_stride,
      frame->shm_format);
  wl_shm_pool_destroy(pool);
  close(fd);
  if (buffer == NULL) {
    munmap(data, size);
    return -1;
  }

  capture->frame_buffer = buffer;
  capture->shm_data = data;
  capture->shm_size = size;
  capture->frame_is_dmabuf = 0;
  capture->frame_format = frame->shm_format;
  capture->frame_width = (int32_t)frame->shm_width;
  capture->frame_height = (int32_t)frame->shm_height;
  capture->frame_stride = (int32_t)frame->shm_stride;
  return 0;
}

static int ensure_dmabuf_buffer(struct capture *capture, struct frame *frame) {
  if (!frame->have_dmabuf || capture->dmabuf == NULL || !capture->gpu.ready) {
    return -1;
  }
  if (capture->frame_buffer != NULL && capture->frame_is_dmabuf &&
      capture->frame_format == frame->dmabuf_format &&
      capture->frame_width == (int32_t)frame->dmabuf_width &&
      capture->frame_height == (int32_t)frame->dmabuf_height) {
    return 0;
  }
  destroy_frame_buffer(capture);

  struct gbm_bo *bo = gbm_bo_create(capture->gpu.gbm, frame->dmabuf_width, frame->dmabuf_height,
                                    frame->dmabuf_format, GBM_BO_USE_LINEAR | GBM_BO_USE_RENDERING);
  if (bo == NULL) {
    return -1;
  }
  int fd = gbm_bo_get_fd(bo);
  if (fd < 0) {
    gbm_bo_destroy(bo);
    return -1;
  }
  struct zwp_linux_buffer_params_v1 *params =
      zwp_linux_dmabuf_v1_create_params(capture->dmabuf);
  if (params == NULL) {
    close(fd);
    gbm_bo_destroy(bo);
    return -1;
  }
  uint64_t modifier = gbm_bo_get_modifier(bo);
  zwp_linux_buffer_params_v1_add(params, fd, 0, gbm_bo_get_offset(bo, 0),
                                 (uint32_t)gbm_bo_get_stride(bo), (uint32_t)(modifier >> 32),
                                 (uint32_t)(modifier & 0xffffffffu));
  struct wl_buffer *buffer = zwp_linux_buffer_params_v1_create_immed(
      params, (int32_t)frame->dmabuf_width, (int32_t)frame->dmabuf_height, frame->dmabuf_format, 0);
  zwp_linux_buffer_params_v1_destroy(params);
  close(fd);
  if (buffer == NULL) {
    gbm_bo_destroy(bo);
    return -1;
  }

  EGLImage image = gpu_image_from_bo(&capture->gpu, bo, frame->dmabuf_format,
                                     (int)frame->dmabuf_width, (int)frame->dmabuf_height);
  if (image == EGL_NO_IMAGE) {
    wl_buffer_destroy(buffer);
    gbm_bo_destroy(bo);
    return -1;
  }

  capture->frame_buffer = buffer;
  capture->frame_bo = bo;
  capture->frame_image = image;
  capture->frame_is_dmabuf = 1;
  capture->frame_format = frame->dmabuf_format;
  capture->frame_width = (int32_t)frame->dmabuf_width;
  capture->frame_height = (int32_t)frame->dmabuf_height;
  capture->frame_stride = 0;
  return 0;
}

static void frame_handle_buffer_done(void *data, struct zwlr_screencopy_frame_v1 *proxy) {
  (void)proxy;
  struct frame *frame = data;
  struct capture *capture = frame->capture;
  if (capture->use_dmabuf && ensure_dmabuf_buffer(capture, frame) != 0) {
    capture->use_dmabuf = 0;
  }
  if (!capture->use_dmabuf && ensure_shm_buffer(capture, frame) != 0) {
    frame->failed = 1;
    return;
  }
  zwlr_screencopy_frame_v1_copy(frame->proxy, capture->frame_buffer);
  frame->copied = 1;
}

static const struct zwlr_screencopy_frame_v1_listener frame_listener = {
    .buffer = frame_handle_buffer,
    .flags = frame_handle_flags,
    .ready = frame_handle_ready,
    .failed = frame_handle_failed,
    .damage = frame_handle_damage,
    .linux_dmabuf = frame_handle_linux_dmabuf,
    .buffer_done = frame_handle_buffer_done,
};

static void frame_release(struct frame *frame) {
  if (frame->proxy != NULL) {
    zwlr_screencopy_frame_v1_destroy(frame->proxy);
  }
}

static int dispatch_until(struct wl_display *display, int64_t deadline_ns, int *done,
                          int *failed) {
  while (!*done && !*failed) {
    if (atomic_load(&g_stop)) {
      return -1;
    }
    int64_t remaining = deadline_ns - now_ns();
    if (remaining <= 0) {
      return 0;
    }
    int timeout = (int)(remaining / 1000000LL);
    if (timeout <= 0) {
      timeout = 1;
    }
    while (wl_display_prepare_read(display) != 0) {
      if (wl_display_dispatch_pending(display) < 0) {
        return -1;
      }
    }
    if (wl_display_flush(display) < 0 && errno != EAGAIN) {
      wl_display_cancel_read(display);
      return -1;
    }
    struct pollfd descriptor = {
        .fd = wl_display_get_fd(display),
        .events = POLLIN,
    };
    int ready = poll(&descriptor, 1, timeout);
    if (ready > 0) {
      if (wl_display_read_events(display) < 0) {
        return -1;
      }
      if (wl_display_dispatch_pending(display) < 0) {
        return -1;
      }
    } else {
      wl_display_cancel_read(display);
      if (ready == 0) {
        return 0;
      }
      if (errno != EINTR) {
        return -1;
      }
    }
  }
  return 0;
}

static void sample_shm(const uint8_t *data, int stride, int source_width, int source_height,
                       uint32_t format, int y_invert, uint8_t *output, int grid_width,
                       int grid_height) {
  int red = 2;
  int blue = 0;
  if (format == WL_SHM_FORMAT_ABGR8888 || format == WL_SHM_FORMAT_XBGR8888) {
    red = 0;
    blue = 2;
  }
  for (int y = 0; y < grid_height; y += 1) {
    int source_y = (int)((int64_t)y * source_height / grid_height);
    if (y_invert) {
      source_y = source_height - 1 - source_y;
    }
    const uint8_t *row = data + (size_t)source_y * (size_t)stride;
    for (int x = 0; x < grid_width; x += 1) {
      int source_x = (int)((int64_t)x * source_width / grid_width);
      const uint8_t *pixel = row + (size_t)source_x * 4;
      uint8_t *target = output + ((size_t)y * grid_width + x) * 4;
      target[0] = pixel[red];
      target[1] = pixel[1];
      target[2] = pixel[blue];
      target[3] = 255;
    }
  }
}

static int capture_frame(struct capture *capture, uint8_t *grid, int grid_width,
                         int grid_height) {
  struct frame frame;
  memset(&frame, 0, sizeof frame);
  frame.capture = capture;
  frame.proxy = zwlr_screencopy_manager_v1_capture_output(capture->manager, 0, capture->output);
  if (frame.proxy == NULL) {
    return -1;
  }
  zwlr_screencopy_frame_v1_add_listener(frame.proxy, &frame_listener, &frame);

  int64_t deadline = now_ns() + (int64_t)JYOTI_FRAME_TIMEOUT_MS * 1000000LL;
  int status = dispatch_until(capture->display, deadline, &frame.ready, &frame.failed);
  if (status < 0 || !frame.ready) {
    frame_release(&frame);
    return -1;
  }

  int rendered = -1;
  if (capture->frame_is_dmabuf && capture->frame_image != EGL_NO_IMAGE) {
    rendered = gpu_render(&capture->gpu, grid_width, grid_height, frame.y_invert, grid);
    if (rendered != 0) {
      capture->use_dmabuf = 0;
    }
  }
  if (rendered != 0 && capture->shm_data != NULL) {
    sample_shm(capture->shm_data, capture->frame_stride, capture->frame_width,
               capture->frame_height, capture->frame_format, frame.y_invert, grid, grid_width,
               grid_height);
    rendered = 0;
  }

  frame_release(&frame);
  return rendered;
}

static void publish_frame(const uint8_t *grid, int width, int height) {
  size_t size = (size_t)width * height * 4;
  uint32_t sequence = atomic_load_explicit(&g_sequence, memory_order_relaxed);
  atomic_store_explicit(&g_sequence, sequence + 1, memory_order_release);
  atomic_thread_fence(memory_order_release);
  memcpy(g_grid, grid, size);
  atomic_store_explicit(&g_captured_at_ms, now_ns() / 1000000LL, memory_order_relaxed);
  atomic_store_explicit(&g_sequence, sequence + 2, memory_order_release);
}

/* ------------------------------------------------------------------ */
/* Thread                                                              */
/* ------------------------------------------------------------------ */

static void capture_registry_global(void *data, struct wl_registry *registry, uint32_t name,
                                    const char *interface, uint32_t version) {
  struct capture *capture = data;
  if (strcmp(interface, wl_output_interface.name) == 0 && capture->outputs.count < JYOTI_MAX_OUTPUTS) {
    uint32_t bound = version < 4 ? version : 4;
    struct wl_output *output = wl_registry_bind(registry, name, &wl_output_interface, bound);
    if (output == NULL) {
      return;
    }
    struct output_entry *entry = &capture->outputs.entries[capture->outputs.count];
    memset(entry, 0, sizeof *entry);
    entry->output = output;
    wl_output_add_listener(output, &output_listener, entry);
    capture->outputs.count += 1;
    attach_xdg_output(&capture->outputs, entry);
    return;
  }
  if (strcmp(interface, zxdg_output_manager_v1_interface.name) == 0 &&
      capture->outputs.xdg_manager == NULL) {
    uint32_t bound = version < 3 ? version : 3;
    capture->outputs.xdg_manager =
        wl_registry_bind(registry, name, &zxdg_output_manager_v1_interface, bound);
    for (int index = 0; index < capture->outputs.count; index += 1) {
      attach_xdg_output(&capture->outputs, &capture->outputs.entries[index]);
    }
    return;
  }
  if (strcmp(interface, zwlr_screencopy_manager_v1_interface.name) == 0 &&
      capture->manager == NULL && version >= 3) {
    capture->manager =
        wl_registry_bind(registry, name, &zwlr_screencopy_manager_v1_interface, 3);
    return;
  }
  if (strcmp(interface, wl_shm_interface.name) == 0 && capture->shm == NULL) {
    capture->shm = wl_registry_bind(registry, name, &wl_shm_interface, 1);
    return;
  }
  if (strcmp(interface, zwp_linux_dmabuf_v1_interface.name) == 0 && capture->dmabuf == NULL &&
      version >= 3) {
    capture->dmabuf = wl_registry_bind(registry, name, &zwp_linux_dmabuf_v1_interface, 3);
  }
}

static const struct wl_registry_listener capture_registry_listener = {
    .global = capture_registry_global,
    .global_remove = registry_global_remove,
};

static struct wl_output *find_output(struct capture *capture, const char *name) {
  for (int index = 0; index < capture->outputs.count; index += 1) {
    struct output_entry *entry = &capture->outputs.entries[index];
    if (strcmp(entry->name, name) == 0) {
      return entry->output;
    }
  }
  return NULL;
}

static int capture_connect(struct capture *capture) {
  if (capture->display != NULL) {
    return 0;
  }
  capture->display = wl_display_connect(NULL);
  if (capture->display == NULL) {
    set_error_kind(3, "Could not connect to Wayland. Start jyoti inside a Hyprland session.");
    return -1;
  }
  capture->registry = wl_display_get_registry(capture->display);
  if (capture->registry == NULL) {
    set_error_kind(3, "Could not reach the Wayland registry.");
    return -1;
  }
  wl_registry_add_listener(capture->registry, &capture_registry_listener, capture);
  if (wl_display_roundtrip(capture->display) < 0 || wl_display_roundtrip(capture->display) < 0) {
    set_error_kind(3, "The Wayland compositor did not answer.");
    return -1;
  }
  capture->output = find_output(capture, g_monitor_name);
  if (capture->output == NULL) {
    set_error_kind(2, "Monitor \"%s\" is not connected.", g_monitor_name);
    return -1;
  }
  if (capture->manager == NULL) {
    set_error("Hyprland did not offer wlr-screencopy v3.");
    return -1;
  }
  if (capture->shm == NULL) {
    set_error("Hyprland did not offer wl_shm.");
    return -1;
  }
  if (!capture->gpu_attempted) {
    capture->gpu_attempted = 1;
    capture->use_dmabuf = capture->dmabuf != NULL && gpu_init(&capture->gpu) == 0;
  } else {
    capture->use_dmabuf = capture->gpu.ready;
  }
  return 0;
}

static void capture_disconnect(struct capture *capture) {
  destroy_frame_buffer(capture);
  free_outputs(&capture->outputs);
  if (capture->manager != NULL) {
    zwlr_screencopy_manager_v1_destroy(capture->manager);
    capture->manager = NULL;
  }
  if (capture->dmabuf != NULL) {
    zwp_linux_dmabuf_v1_destroy(capture->dmabuf);
    capture->dmabuf = NULL;
  }
  if (capture->shm != NULL) {
    wl_shm_destroy(capture->shm);
    capture->shm = NULL;
  }
  if (capture->registry != NULL) {
    wl_registry_destroy(capture->registry);
    capture->registry = NULL;
  }
  if (capture->display != NULL) {
    wl_display_disconnect(capture->display);
    capture->display = NULL;
  }
  capture->output = NULL;
}

static void wait_until(int64_t deadline_ns) {
  pthread_mutex_lock(&g_lock);
  struct timespec deadline;
  clock_gettime(CLOCK_MONOTONIC, &deadline);
  int64_t wait_ns = deadline_ns - now_ns();
  if (wait_ns < 0) {
    wait_ns = 0;
  }
  deadline.tv_sec += (time_t)(wait_ns / 1000000000LL);
  deadline.tv_nsec += (long)(wait_ns % 1000000000LL);
  if (deadline.tv_nsec >= 1000000000L) {
    deadline.tv_sec += 1;
    deadline.tv_nsec -= 1000000000L;
  }
  while (!atomic_load(&g_stop)) {
    int waited = pthread_cond_timedwait(&g_cond, &g_lock, &deadline);
    if (waited == ETIMEDOUT) {
      break;
    }
  }
  pthread_mutex_unlock(&g_lock);
}

static void *capture_thread(void *argument) {
  struct capture *capture = argument;
  int ok = 0;
  if (capture_connect(capture) != 0) {
    goto finish;
  }

  int64_t interval = 1000000000LL / (g_rate_hz > 0 ? g_rate_hz : 10);
  int64_t next = now_ns();
  int64_t backoff = 250000000LL;
  uint8_t *grid = malloc((size_t)g_grid_width * g_grid_height * 4);
  if (grid == NULL) {
    set_error("Could not allocate the capture grid.");
    goto finish;
  }

  ok = 1;
  pthread_mutex_lock(&g_lock);
  g_init_state = 1;
  pthread_cond_broadcast(&g_cond);
  pthread_mutex_unlock(&g_lock);

  int failures = 0;
  while (!atomic_load(&g_stop)) {
    if (capture->display == NULL) {
      wait_until(now_ns() + backoff);
      if (atomic_load(&g_stop)) {
        break;
      }
      if (capture_connect(capture) == 0) {
        failures = 0;
        backoff = 250000000LL;
        next = now_ns();
      } else {
        capture_disconnect(capture);
        backoff *= 2;
        if (backoff > 2000000000LL) {
          backoff = 2000000000LL;
        }
      }
      continue;
    }
    if (capture_frame(capture, grid, g_grid_width, g_grid_height) == 0) {
      publish_frame(grid, g_grid_width, g_grid_height);
      failures = 0;
      backoff = 250000000LL;
    } else {
      failures += 1;
      if (failures >= 5) {
        capture_disconnect(capture);
      }
    }
    next += interval;
    int64_t current = now_ns();
    if (next <= current) {
      next = current + interval;
    }
    wait_until(next);
  }
  free(grid);

finish:
  if (!ok) {
    pthread_mutex_lock(&g_lock);
    g_init_state = -1;
    pthread_cond_broadcast(&g_cond);
    pthread_mutex_unlock(&g_lock);
  }
  capture_disconnect(capture);
  if (capture->gpu.drm_fd >= 0 || capture->gpu.gbm != NULL ||
      capture->gpu.display != EGL_NO_DISPLAY) {
    gpu_finish(&capture->gpu);
  }
  return NULL;
}

/* ------------------------------------------------------------------ */
/* Public API                                                          */
/* ------------------------------------------------------------------ */

JYOTI_API int32_t jyoti_capture_list_monitors(jyoti_monitor *output, int32_t capacity) {
  struct wl_display *display = wl_display_connect(NULL);
  if (display == NULL) {
    set_error_kind(3, "Could not connect to Wayland. Start jyoti inside a Hyprland session.");
    return -1;
  }
  struct output_list list;
  memset(&list, 0, sizeof list);
  if (collect_outputs(display, &list) != 0) {
    set_error_kind(3, "The Wayland compositor did not answer.");
    free_outputs(&list);
    wl_display_disconnect(display);
    return -1;
  }
  int32_t count = list.count;
  if (output != NULL) {
    for (int index = 0; index < list.count && index < capacity; index += 1) {
      monitor_from_entry(&list.entries[index], &output[index]);
    }
  }
  free_outputs(&list);
  wl_display_disconnect(display);
  return count;
}

JYOTI_API int32_t jyoti_capture_start(const char *monitor, int32_t width, int32_t height,
                                      int32_t rate_hz) {
  if (monitor == NULL || monitor[0] == '\0') {
    set_error_kind(2, "Choose a monitor first.");
    return -2;
  }
  if (width <= 0 || height <= 0 || width > JYOTI_MAX_GRID || height > JYOTI_MAX_GRID) {
    set_error("The capture grid must be between 1 and %d pixels.", JYOTI_MAX_GRID);
    return -1;
  }
  if (rate_hz <= 0 || rate_hz > 60) {
    set_error("The capture rate must be between 1 and 60 Hz.");
    return -1;
  }
  if (atomic_load(&g_running)) {
    set_error("Capture is already running.");
    return -1;
  }

  pthread_once(&g_cond_once, init_cond);

  g_grid_width = width;
  g_grid_height = height;
  g_rate_hz = rate_hz;
  copy_text(g_monitor_name, sizeof g_monitor_name, monitor);
  g_grid = malloc((size_t)width * height * 4);
  if (g_grid == NULL) {
    set_error("Could not allocate the capture grid.");
    return -1;
  }
  atomic_store(&g_sequence, 0);
  atomic_store(&g_captured_at_ms, 0);
  atomic_store(&g_stop, 0);
  atomic_store(&g_running, 1);

  pthread_mutex_lock(&g_lock);
  g_init_state = 0;
  g_error_kind = 4;
  g_error[0] = '\0';
  pthread_mutex_unlock(&g_lock);

  static struct capture capture;
  memset(&capture, 0, sizeof capture);
  capture.gpu.drm_fd = -1;
  capture.gpu.display = EGL_NO_DISPLAY;
  if (pthread_create(&g_thread, NULL, capture_thread, &capture) != 0) {
    atomic_store(&g_running, 0);
    free(g_grid);
    g_grid = NULL;
    set_error("Could not start the capture thread.");
    return -1;
  }

  pthread_mutex_lock(&g_lock);
  int64_t deadline = now_ns() + (int64_t)JYOTI_INIT_TIMEOUT_MS * 1000000LL;
  while (g_init_state == 0) {
    struct timespec wake;
    clock_gettime(CLOCK_MONOTONIC, &wake);
    wake.tv_sec += 1;
    int waited = pthread_cond_timedwait(&g_cond, &g_lock, &wake);
    if (waited == ETIMEDOUT && now_ns() > deadline) {
      break;
    }
  }
  int state = g_init_state;
  pthread_mutex_unlock(&g_lock);

  if (state != 1) {
    atomic_store(&g_stop, 1);
    pthread_mutex_lock(&g_lock);
    pthread_cond_broadcast(&g_cond);
    pthread_mutex_unlock(&g_lock);
    pthread_join(g_thread, NULL);
    atomic_store(&g_running, 0);
    free(g_grid);
    g_grid = NULL;
    if (g_error[0] == '\0') {
      set_error("The capture thread did not start.");
    }
    int kind = g_error_kind;
    return (kind == 2 || kind == 3) ? -kind : -1;
  }
  return 0;
}

JYOTI_API void jyoti_capture_stop(void) {
  if (!atomic_load(&g_running)) {
    return;
  }
  atomic_store(&g_stop, 1);
  pthread_mutex_lock(&g_lock);
  pthread_cond_broadcast(&g_cond);
  pthread_mutex_unlock(&g_lock);
  pthread_join(g_thread, NULL);
  atomic_store(&g_running, 0);
  free(g_grid);
  g_grid = NULL;
  atomic_store(&g_sequence, 0);
  atomic_store(&g_captured_at_ms, 0);
}

JYOTI_API uint64_t jyoti_capture_latest(uint8_t *output, uint64_t capacity,
                                        jyoti_frame_info *info) {
  if (!atomic_load(&g_running) || g_grid == NULL || output == NULL) {
    return 0;
  }
  size_t size = (size_t)g_grid_width * g_grid_height * 4;
  if (capacity < size) {
    return 0;
  }
  uint32_t first = atomic_load_explicit(&g_sequence, memory_order_acquire);
  if (first == 0 || (first & 1u) != 0) {
    return 0;
  }
  memcpy(output, g_grid, size);
  atomic_thread_fence(memory_order_acquire);
  uint32_t second = atomic_load_explicit(&g_sequence, memory_order_acquire);
  if (first != second) {
    return 0;
  }
  int64_t captured = atomic_load_explicit(&g_captured_at_ms, memory_order_relaxed);
  if (info != NULL) {
    info->width = g_grid_width;
    info->height = g_grid_height;
    info->age_ms = now_ns() / 1000000LL - captured;
    if (info->age_ms < 0) {
      info->age_ms = 0;
    }
  }
  return second;
}

JYOTI_API const char *jyoti_capture_error(void) { return g_error; }
