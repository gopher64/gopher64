// macos_bookmark.m — security-scoped bookmarks for the macOS App Sandbox.
//
// Lets a user-picked ROM folder (chosen via the open panel) stay accessible
// across app launches. Compiled only on macOS (see build.rs) with -fobjc-arc.
//
// Entitlement note: creating security-scoped bookmarks requires the app to be
// sandboxed with com.apple.security.files.user-selected.read-write (the default
// for a file-accessing sandboxed app); the bundle must carry that entitlement.

#import <Foundation/Foundation.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// Create a security-scoped bookmark for `path`. On success returns a malloc'd
// buffer (free with gopher64_bookmark_free) and writes its length to *out_len.
// Returns NULL on failure.
uint8_t *gopher64_bookmark_create(const char *path, size_t *out_len) {
    @autoreleasepool {
        NSString *p = [[NSFileManager defaultManager]
            stringWithFileSystemRepresentation:path
                                        length:strlen(path)];
        if (!p) {
            return NULL;
        }
        NSURL *url = [NSURL fileURLWithPath:p];
        NSError *err = nil;
        NSData *data = [url bookmarkDataWithOptions:NSURLBookmarkCreationWithSecurityScope
                     includingResourceValuesForKeys:nil
                                      relativeToURL:nil
                                              error:&err];
        if (!data) {
            return NULL;
        }
        size_t len = (size_t)data.length;
        uint8_t *buf = (uint8_t *)malloc(len);
        if (!buf) {
            return NULL;
        }
        memcpy(buf, data.bytes, len);
        *out_len = len;
        return buf;
    }
}

// Resolve a security-scoped bookmark and begin accessing the resource (the scope
// stays open until the process exits). Returns the resolved filesystem path as a
// malloc'd C string (free with gopher64_string_free), or NULL on failure. Sets
// *out_stale to 1 if the bookmark is stale and should be recreated.
char *gopher64_bookmark_resolve(const uint8_t *bytes, size_t len, int *out_stale) {
    @autoreleasepool {
        NSData *data = [NSData dataWithBytes:bytes length:len];
        BOOL stale = NO;
        NSError *err = nil;
        NSURL *url = [NSURL URLByResolvingBookmarkData:data
                                               options:NSURLBookmarkResolutionWithSecurityScope
                                         relativeToURL:nil
                                   bookmarkDataIsStale:&stale
                                                 error:&err];
        if (!url) {
            return NULL;
        }
        if (out_stale) {
            *out_stale = stale ? 1 : 0;
        }
        // Begin accessing; intentionally not balanced with a stop — the folder
        // stays available for the whole session and the OS releases it on exit.
        [url startAccessingSecurityScopedResource];
        const char *rep = url.fileSystemRepresentation;
        if (!rep) {
            return NULL;
        }
        return strdup(rep);
    }
}

void gopher64_bookmark_free(uint8_t *buf) {
    free(buf);
}

void gopher64_string_free(char *s) {
    free(s);
}
