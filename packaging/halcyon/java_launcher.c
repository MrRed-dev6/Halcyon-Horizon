/*
 * halcyon_java: `java` for the Android app (Halcyon).
 *
 * Android does not let an app run files from its data folder, but it does run the files the app
 * ships in its native library folder. The app ships this program as libhalcyon_java.so; the JRE
 * itself lives in the app's data folder. This loads the JRE's libjli.so and calls JLI_Launch, which
 * is what the JDK's own `java` program does.
 *
 *   HALCYON_JRE_HOME  the JRE's folder (the one with lib/libjli.so); JAVA_HOME is used if it is unset
 *   HALCYON_TMP       java.io.tmpdir, because Android has no /tmp
 */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int (*jli_launch_t)(int argc, char **argv, int jargc, const char **jargv, int appclassc,
                            const char **appclassv, const char *fullversion, const char *dotversion,
                            const char *pname, const char *lname, unsigned char javaargs,
                            unsigned char cpwildcard, unsigned char javaw, int ergo);

static void *load(const char *home, const char *relative) {
    char path[4096];
    snprintf(path, sizeof path, "%s/%s", home, relative);
    void *handle = dlopen(path, RTLD_NOW | RTLD_GLOBAL);
    if (!handle) {
        fprintf(stderr, "halcyon_java: cannot load %s: %s\n", path, dlerror());
    }
    return handle;
}

int main(int argc, char **argv) {
    const char *home = getenv("HALCYON_JRE_HOME");
    if (!home || !*home) home = getenv("JAVA_HOME");
    if (!home || !*home) {
        fprintf(stderr, "halcyon_java: HALCYON_JRE_HOME is not set\n");
        return 2;
    }

    /* libjvm first, so its own dependencies and any error show up here with a clear name. */
    if (!load(home, "lib/server/libjvm.so")) return 3;
    void *jli = load(home, "lib/libjli.so");
    if (!jli) return 3;
    jli_launch_t launch = (jli_launch_t)dlsym(jli, "JLI_Launch");
    if (!launch) {
        fprintf(stderr, "halcyon_java: JLI_Launch not found: %s\n", dlerror());
        return 4;
    }

    /* argv for the launcher: the program name, java.io.tmpdir, then the caller's arguments. */
    const char *tmp = getenv("HALCYON_TMP");
    char **args = calloc((size_t)argc + 3, sizeof *args);
    if (!args) return 5;
    int n = 0;
    args[n++] = argv[0];
    char tmpopt[4096];
    if (tmp && *tmp) {
        snprintf(tmpopt, sizeof tmpopt, "-Djava.io.tmpdir=%s", tmp);
        args[n++] = tmpopt;
    }
    for (int i = 1; i < argc; i++) args[n++] = argv[i];
    args[n] = NULL;

    return launch(n, args, 0, NULL, 0, NULL, "25", "25", "java", "openjdk", 0, 1, 0, 0);
}
