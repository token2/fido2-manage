#define _GNU_SOURCE
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <dlfcn.h>
#include <spawn.h>

/* WebKitGTK 4.1 ignores WEBKIT_EXEC_PATH (honored only in developer-mode
 * builds), so a relocated (AppImage) build tries to exec its network/web/GPU
 * helper at the BUILD host's absolute path, e.g.
 * /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/WebKitNetworkProcess, which does not
 * exist on other distros (Fedora, etc.). We intercept the spawn/exec calls and
 * rewrite any path under a webkit2gtk-4.1 directory to the bundled copy under
 * WEBKIT_EXEC_PATH. Only matching, existing paths are touched; everything else
 * passes through unchanged. */
static const char *rewrite(const char *path, char *buf, size_t n) {
    if (!path) return path;
    const char *marker = "/webkit2gtk-4.1/";
    const char *m = strstr(path, marker);
    const char *base = getenv("WEBKIT_EXEC_PATH");
    if (!m || !base || !*base) return path;
    const char *rest = m + strlen(marker);
    if (snprintf(buf, n, "%s/%s", base, rest) >= (int)n) return path;
    if (access(buf, F_OK) != 0) return path;
    return buf;
}

int posix_spawn(pid_t *pid, const char *path,
                const posix_spawn_file_actions_t *fa, const posix_spawnattr_t *at,
                char *const argv[], char *const envp[]) {
    static int (*real)(pid_t*, const char*, const posix_spawn_file_actions_t*,
                       const posix_spawnattr_t*, char *const[], char *const[]);
    char buf[4096];
    if (!real) real = dlsym(RTLD_NEXT, "posix_spawn");
    return real(pid, rewrite(path, buf, sizeof buf), fa, at, argv, envp);
}
int posix_spawnp(pid_t *pid, const char *file,
                 const posix_spawn_file_actions_t *fa, const posix_spawnattr_t *at,
                 char *const argv[], char *const envp[]) {
    static int (*real)(pid_t*, const char*, const posix_spawn_file_actions_t*,
                       const posix_spawnattr_t*, char *const[], char *const[]);
    char buf[4096];
    if (!real) real = dlsym(RTLD_NEXT, "posix_spawnp");
    return real(pid, rewrite(file, buf, sizeof buf), fa, at, argv, envp);
}
int execv(const char *path, char *const argv[]) {
    static int (*real)(const char*, char *const[]);
    char buf[4096];
    if (!real) real = dlsym(RTLD_NEXT, "execv");
    return real(rewrite(path, buf, sizeof buf), argv);
}
int execvp(const char *file, char *const argv[]) {
    static int (*real)(const char*, char *const[]);
    char buf[4096];
    if (!real) real = dlsym(RTLD_NEXT, "execvp");
    return real(rewrite(file, buf, sizeof buf), argv);
}
int execve(const char *path, char *const argv[], char *const envp[]) {
    static int (*real)(const char*, char *const[], char *const[]);
    char buf[4096];
    if (!real) real = dlsym(RTLD_NEXT, "execve");
    return real(rewrite(path, buf, sizeof buf), argv, envp);
}
int execvpe(const char *file, char *const argv[], char *const envp[]) {
    static int (*real)(const char*, char *const[], char *const[]);
    char buf[4096];
    if (!real) real = dlsym(RTLD_NEXT, "execvpe");
    return real(rewrite(file, buf, sizeof buf), argv, envp);
}
