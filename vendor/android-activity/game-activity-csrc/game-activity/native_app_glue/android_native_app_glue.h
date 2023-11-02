/*
 * Copyright (C) 2021 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#pragma once

/**
 * @addtogroup android_native_app_glue Native App Glue library
 * The glue library to interface your game loop with GameActivity.
 * @{
 */

#include <android/configuration.h>
#include <android/looper.h>
#include <poll.h>
#include <pthread.h>
#include <sched.h>

#include "game-activity/GameActivity.h"

#if (defined NATIVE_APP_GLUE_MAX_NUM_MOTION_EVENTS_OVERRIDE)
#define NATIVE_APP_GLUE_MAX_NUM_MOTION_EVENTS \
    NATIVE_APP_GLUE_MAX_NUM_MOTION_EVENTS_OVERRIDE
#else
#define NATIVE_APP_GLUE_MAX_NUM_MOTION_EVENTS 16
#endif

#if (defined NATIVE_APP_GLUE_MAX_HISTORICAL_POINTER_SAMPLES_OVERRIDE)
#define NATIVE_APP_GLUE_MAX_HISTORICAL_POINTER_SAMPLES \
    NATIVE_APP_GLUE_MAX_HISTORICAL_POINTER_SAMPLES_OVERRIDE
#else
#define NATIVE_APP_GLUE_MAX_HISTORICAL_POINTER_SAMPLES 64
#endif

#if (defined NATIVE_APP_GLUE_MAX_NUM_KEY_EVENTS_OVERRIDE)
#define NATIVE_APP_GLUE_MAX_NUM_KEY_EVENTS \
    NATIVE_APP_GLUE_MAX_NUM_KEY_EVENTS_OVERRIDE
#else
#define NATIVE_APP_GLUE_MAX_NUM_KEY_EVENTS 4
#endif

#ifdef __cplusplus
extern "C" {
#endif

/**
 * The GameActivity interface provided by <game-activity/GameActivity.h>
 * is based on a set of application-provided callbacks that will be called
 * by the Activity's main thread when certain events occur.
 *
 * This means that each one of this callbacks _should_ _not_ block, or they
 * risk having the system force-close the application. This programming
 * model is direct, lightweight, but constraining.
 *
 * The 'android_native_app_glue' static library is used to provide a different
 * execution model where the application can implement its own main event
 * loop in a different thread instead. Here's how it works:
 *
 * 1/ The application must provide a function named "android_main()" that
 *    will be called when the activity is created, in a new thread that is
 *    distinct from the activity's main thread.
 *
 * 2/ android_main() receives a pointer to a valid "android_app" structure
 *    that contains references to other important objects, e.g. the
 *    GameActivity obejct instance the application is running in.
 *
 * 3/ the "android_app" object holds an ALooper instance that already
 *    listens to activity lifecycle events (e.g. "pause", "resume").
 *    See APP_CMD_XXX declarations below.
 *
 *    This corresponds to an ALooper identifier returned by
 *    ALooper_pollOnce with value LOOPER_ID_MAIN.
 *
 *    Your application can use the same ALooper to listen to additional
 *    file-descriptors.  They can either be callback based, or with return
 *    identifiers starting with LOOPER_ID_USER.
 *
 * 4/ Whenever you receive a LOOPER_ID_MAIN event,
 *    the returned data will point to an android_poll_source structure.  You
 *    can call the process() function on it, and fill in android_app->onAppCmd
 *    to be called for your own processing of the event.
 *
 *    Alternatively, you can call the low-level functions to read and process
 *    the data directly...  look at the process_cmd() and process_input()
 *    implementations in the glue to see how to do this.
 *
 * See the sample named "native-activity" that comes with the NDK with a
 * full usage example.  Also look at the documentation of GameActivity.
 */

struct android_app;

/**
 * Data associated with an ALooper fd that will be returned as the "outData"
 * when that source has data ready.
 */
struct android_poll_source {
    /**
     * The identifier of this source.  May be LOOPER_ID_MAIN or
     * LOOPER_ID_INPUT.
     */
    int32_t id;

    /** The android_app this ident is associated with. */
    struct android_app* app;

    /**
     * Function to call to perform the standard processing of data from
     * this source.
     */
    void (*process)(struct android_app* app,
                    struct android_poll_source* source);
};

struct android_input_buffer {
    /**
     * Pointer to a read-only array of pointers to GameActivityMotionEvent.
     * Only the first motionEventsCount events are valid.
     */
    GameActivityMotionEvent motionEvents[NATIVE_APP_GLUE_MAX_NUM_MOTION_EVENTS];

    /**
     * The number of valid motion events in `motionEvents`.
     */
    uint64_t motionEventsCount;

    /**
     * Pointer to a read-only array of pointers to GameActivityHistoricalPointerAxes.
     *
     * Only the first historicalSamplesCount samples are valid.
     * Refer to event->historicalStart, event->pointerCount and event->historicalCount
     * to access the specific samples that relate to an event.
     *
     * Each slice of samples for one event has a length of
     * (event->pointerCount and event->historicalCount) and is in pointer-major
     * order so the historic samples for each pointer are contiguous.
     * E.g. you would access historic sample index 3 for pointer 2 of an event with:
     *
     *   historicalAxisSamples[event->historicalStart + (event->historicalCount * 2) + 3];
     */
    GameActivityHistoricalPointerAxes historicalAxisSamples[NATIVE_APP_GLUE_MAX_HISTORICAL_POINTER_SAMPLES];

    /**
     * The number of valid historical samples in `historicalAxisSamples`.
     */
    uint64_t historicalSamplesCount;

    /**
     * Pointer to a read-only array of pointers to GameActivityKeyEvent.
     * Only the first keyEventsCount events are valid.
     */
    GameActivityKeyEvent keyEvents[NATIVE_APP_GLUE_MAX_NUM_KEY_EVENTS];

    /**
     * The number of valid "Key" events in `keyEvents`.
     */
    uint64_t keyEventsCount;
};

/**
 * Function pointer declaration for the filtering of key events.
 * A function with this signature should be passed to
 * android_app_set_key_event_filter and return false for any events that should
 * not be handled by android_native_app_glue. These events will be handled by
 * the system instead.
 */
typedef bool (*android_key_event_filter)(const GameActivityKeyEvent*);

/**
 * Function pointer definition for the filtering of motion events.
 * A function with this signature should be passed to
 * android_app_set_motion_event_filter and return false for any events that
 * should not be handled by android_native_app_glue. These events will be
 * handled by the system instead.
 */
typedef bool (*android_motion_event_filter)(const GameActivityMotionEvent*);

/**
 * This is the interface for the standard glue code of a threaded
 * application.  In this model, the application's code is running
 * in its own thread separate from the main thread of the process.
 * It is not required that this thread be associated with the Java
 * VM, although it will need to be in order to make JNI calls any
 * Java objects.
 */
struct android_app {
    /**
     * An optional pointer to application-defined state.
     */
    void* userData;

    /**
     * A required callback for processing main app commands (`APP_CMD_*`).
     * This is called each frame if there are app commands that need processing.
     */
    void (*onAppCmd)(struct android_app* app, int32_t cmd);

    /** The GameActivity object instance that this app is running in. */
    GameActivity* activity;

    /** The current configuration the app is running in. */
    AConfiguration* config;

    /**
     * The last activity saved state, as provided at creation time.
     * It is NULL if there was no state.  You can use this as you need; the
     * memory will remain around until you call android_app_exec_cmd() for
     * APP_CMD_RESUME, at which point it will be freed and savedState set to
     * NULL. These variables should only be changed when processing a
     * APP_CMD_SAVE_STATE, at which point they will be initialized to NULL and
     * you can malloc your state and place the information here.  In that case
     * the memory will be freed for you later.
     */
    void* savedState;

    /**
     * The size of the activity saved state. It is 0 if `savedState` is NULL.
     */
    size_t savedStateSize;

    /** The ALooper associated with the app's thread. */
    ALooper* looper;

    /** When non-NULL, this is the window surface that the app can draw in. */
    ANativeWindow* window;

    /**
     * Current content rectangle of the window; this is the area where the
     * window's content should be placed to be seen by the user.
     */
    ARect contentRect;

    /**
     * Current state of the app's activity.  May be either APP_CMD_START,
     * APP_CMD_RESUME, APP_CMD_PAUSE, or APP_CMD_STOP.
     */
    int activityState;

    /**
     * This is non-zero when the application's GameActivity is being
     * destroyed and waiting for the app thread to complete.
     */
    int destroyRequested;

#define NATIVE_APP_GLUE_MAX_INPUT_BUFFERS 2

    /**
     * This is used for buffering input from GameActivity. Once ready, the
     * application thread switches the buffers and processes what was
     * accumulated.
     */
    struct android_input_buffer inputBuffers[NATIVE_APP_GLUE_MAX_INPUT_BUFFERS];

    int currentInputBuffer;

    /**
     * 0 if no text input event is outstanding, 1 if it is.
     * Use `GameActivity_getTextInputState` to get information
     * about the text entered by the user.
     */
    int textInputState;

    // Below are "private" implementation of the glue code.
    /** @cond INTERNAL */

    pthread_mutex_t mutex;
    pthread_cond_t cond;

    int msgread;
    int msgwrite;

    pthread_t thread;

    struct android_poll_source cmdPollSource;

    int running;
    int stateSaved;
    int destroyed;
    int redrawNeeded;
    ANativeWindow* pendingWindow;
    ARect pendingContentRect;

    android_key_event_filter keyEventFilter;
    android_motion_event_filter motionEventFilter;

    // When new input is received we set both of these flags and use the looper to
    // wake up the application mainloop.
    //
    // To avoid spamming the mainloop with wake ups from lots of input though we
    // don't sent a wake up if the inputSwapPending flag is already set. (i.e.
    // we already expect input to be processed in a finite amount of time due to
    // our previous wake up)
    //
    // When a wake up is received then we will check this flag (clearing it
    // at the same time). If it was set then an InputAvailable event is sent to
    // the application - which should lead to all input being processed within
    // a finite amount of time.
    //
    // The next time android_app_swap_input_buffers is called, both flags will be
    // cleared.
    //
    // NB: both of these should only be read with the app mutex held
    bool inputAvailableWakeUp;
    bool inputSwapPending;

    /** @endcond */
};

/**
 * Looper ID of commands coming from the app's main thread, an AInputQueue or
 * user-defined sources.
 */
enum NativeAppGlueLooperId {
    /**
     * Looper data ID of commands coming from the app's main thread, which
     * is returned as an identifier from ALooper_pollOnce().  The data for this
     * identifier is a pointer to an android_poll_source structure.
     * These can be retrieved and processed with android_app_read_cmd()
     * and android_app_exec_cmd().
     */
    LOOPER_ID_MAIN = 1,

    /**
     * Unused. Reserved for future use when usage of AInputQueue will be
     * supported.
     */
    LOOPER_ID_INPUT = 2,

    /**
     * Start of user-defined ALooper identifiers.
     */
    LOOPER_ID_USER = 3,
};

/**
 * Commands passed from the application's main Java thread to the game's thread.
 */
enum NativeAppGlueAppCmd {
    /**
     * Unused. Reserved for future use when usage of AInputQueue will be
     * supported.
     */
    UNUSED_APP_CMD_INPUT_CHANGED,

    /**
     * Command from main thread: a new ANativeWindow is ready for use.  Upon
     * receiving this command, android_app->window will contain the new window
     * surface.
     */
    APP_CMD_INIT_WINDOW,

    /**
     * Command from main thread: the existing ANativeWindow needs to be
     * terminated.  Upon receiving this command, android_app->window still
     * contains the existing window; after calling android_app_exec_cmd
     * it will be set to NULL.
     */
    APP_CMD_TERM_WINDOW,

    /**
     * Command from main thread: the current ANativeWindow has been resized.
     * Please redraw with its new size.
     */
    APP_CMD_WINDOW_RESIZED,

    /**
     * Command from main thread: the system needs that the current ANativeWindow
     * be redrawn.  You should redraw the window before handing this to
     * android_app_exec_cmd() in order to avoid transient drawing glitches.
     */
    APP_CMD_WINDOW_REDRAW_NEEDED,

    /**
     * Command from main thread: the content area of the window has changed,
     * such as from the soft input window being shown or hidden.  You can
     * find the new content rect in android_app::contentRect.
     */
    APP_CMD_CONTENT_RECT_CHANGED,

    /**
     * Command from main thread: the app's activity window has gained
     * input focus.
     */
    APP_CMD_GAINED_FOCUS,

    /**
     * Command from main thread: the app's activity window has lost
     * input focus.
     */
    APP_CMD_LOST_FOCUS,

    /**
     * Command from main thread: the current device configuration has changed.
     */
    APP_CMD_CONFIG_CHANGED,

    /**
     * Command from main thread: the system is running low on memory.
     * Try to reduce your memory use.
     */
    APP_CMD_LOW_MEMORY,

    /**
     * Command from main thread: the app's activity has been started.
     */
    APP_CMD_START,

    /**
     * Command from main thread: the app's activity has been resumed.
     */
    APP_CMD_RESUME,

    /**
     * Command from main thread: the app should generate a new saved state
     * for itself, to restore from later if needed.  If you have saved state,
     * allocate it with malloc and place it in android_app.savedState with
     * the size in android_app.savedStateSize.  The will be freed for you
     * later.
     */
    APP_CMD_SAVE_STATE,

    /**
     * Command from main thread: the app's activity has been paused.
     */
    APP_CMD_PAUSE,

    /**
     * Command from main thread: the app's activity has been stopped.
     */
    APP_CMD_STOP,

    /**
     * Command from main thread: the app's activity is being destroyed,
     * and waiting for the app thread to clean up and exit before proceeding.
     */
    APP_CMD_DESTROY,

    /**
     * Command from main thread: the app's insets have changed.
     */
    APP_CMD_WINDOW_INSETS_CHANGED,

};

/**
 * Call when ALooper_pollAll() returns LOOPER_ID_MAIN, reading the next
 * app command message.
 */
int8_t android_app_read_cmd(struct android_app* android_app);

/**
 * Call with the command returned by android_app_read_cmd() to do the
 * initial pre-processing of the given command.  You can perform your own
 * actions for the command after calling this function.
 */
void android_app_pre_exec_cmd(struct android_app* android_app, int8_t cmd);

/**
 * Call with the command returned by android_app_read_cmd() to do the
 * final post-processing of the given command.  You must have done your own
 * actions for the command before calling this function.
 */
void android_app_post_exec_cmd(struct android_app* android_app, int8_t cmd);

/**
 * Call this before processing input events to get the events buffer.
 * The function returns NULL if there are no events to process.
 */
struct android_input_buffer* android_app_swap_input_buffers(
    struct android_app* android_app);

/**
 * Clear the array of motion events that were waiting to be handled, and release
 * each of them.
 *
 * This method should be called after you have processed the motion events in
 * your game loop. You should handle events at each iteration of your game loop.
 */
void android_app_clear_motion_events(struct android_input_buffer* inputBuffer);

/**
 * Clear the array of key events that were waiting to be handled, and release
 * each of them.
 *
 * This method should be called after you have processed the key up events in
 * your game loop. You should handle events at each iteration of your game loop.
 */
void android_app_clear_key_events(struct android_input_buffer* inputBuffer);

/**
 * This is a springboard into the Rust glue layer that wraps calling the
 * main entry for the app itself.
 */
extern void _rust_glue_entry(struct android_app* app);

/**
 * Set the filter to use when processing key events.
 * Any events for which the filter returns false will be ignored by
 * android_native_app_glue. If filter is set to NULL, no filtering is done.
 *
 * The default key filter will filter out volume and camera button presses.
 */
void android_app_set_key_event_filter(struct android_app* app,
                                      android_key_event_filter filter);

/**
 * Set the filter to use when processing touch and motion events.
 * Any events for which the filter returns false will be ignored by
 * android_native_app_glue. If filter is set to NULL, no filtering is done.
 *
 * Note that the default motion event filter will only allow touchscreen events
 * through, in order to mimic NativeActivity's behaviour, so for controller
 * events to be passed to the app, set the filter to NULL.
 */
void android_app_set_motion_event_filter(struct android_app* app,
                                         android_motion_event_filter filter);

/**
 * Determines if a looper wake up was due to new input becoming available
 */
bool android_app_input_available_wake_up(struct android_app* app);

void GameActivity_onCreate_C(GameActivity* activity, void* savedState,
                            size_t savedStateSize);
#ifdef __cplusplus
}
#endif

/** @} */
