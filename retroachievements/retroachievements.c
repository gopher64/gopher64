#include "retroachievements.h"
#include "../parallel-rdp/interface.hpp"
#include <rc_client.h>
#include <rc_consoles.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void rust_server_call(const char *url, const char *post_data,
                      const char *content_type,
                      rc_client_server_callback_t callback,
                      void *callback_data);
void store_retroachievements_credentials(const char *username,
                                         const char *token, void *userdata);

void notify_load_game(void *userdata);

static rc_client_t *g_client = NULL;
static const uint8_t *g_dmem = NULL;
static size_t g_dmem_size = 0;
static bool g_game_loaded = false;
static bool g_user_logged_in = false;
static bool g_challenge = false;
static bool g_leaderboard = false;
static const char *g_username = NULL;
static const char *g_token = NULL;
static char load_game_error_message[512];
static rc_client_leaderboard_list_t *g_leaderboard_list = NULL;

static uint32_t read_memory(uint32_t address, uint8_t *buffer,
                            uint32_t num_bytes, rc_client_t *client) {
  if (address + num_bytes > g_dmem_size)
    memset(buffer, 0, num_bytes);
  else
    memcpy(buffer, &g_dmem[address], num_bytes);
  return num_bytes;
}

static void server_call(const rc_api_request_t *request,
                        rc_client_server_callback_t callback,
                        void *callback_data, rc_client_t *client) {
  rust_server_call(request->url, request->post_data, request->content_type,
                   callback, callback_data);
}

void ra_http_callback(const char *content, size_t content_size, int status_code,
                      void *callback, void *callback_data) {
  // Prepare a data object to pass the HTTP response to the callback
  rc_api_server_response_t server_response;
  memset(&server_response, 0, sizeof(server_response));
  server_response.body = content;
  server_response.body_length = content_size;
  server_response.http_status_code = status_code;

  // handle non-http errors (socket timeout, no internet available, etc)
  if (status_code == 0) {
    // Let rc_client know the error was not catastrophic and could be retried.
    // It may decide to retry or just immediately pass the error to the
    // callback. To prevent possible retries, use
    // RC_API_SERVER_RESPONSE_CLIENT_ERROR.
    server_response.http_status_code =
        RC_API_SERVER_RESPONSE_RETRYABLE_CLIENT_ERROR;
  }

  ((rc_client_server_callback_t)callback)(&server_response, callback_data);
}

static void log_message(const char *message, const rc_client_t *client) {
  printf("RetroAchievements: %s\n", message);
}

static void login_callback(int result, const char *error_message,
                           rc_client_t *client, void *userdata) {
  // If not successful, just report the error and bail.
  if (result != RC_OK) {
    snprintf(load_game_error_message, sizeof(load_game_error_message),
             "RA login failed: %s", error_message);
    store_retroachievements_credentials(NULL, NULL, userdata);
    return;
  } else {
    memset(load_game_error_message, 0, sizeof(load_game_error_message));
  }

  // Login was successful. Capture the token for future logins so we don't have
  // to store the password anywhere.
  const rc_client_user_t *user = rc_client_get_user_info(client);
  store_retroachievements_credentials(user->username, user->token, userdata);

  g_username = user->username;
  g_token = user->token;
  g_user_logged_in = true;
}

bool ra_is_user_logged_in() { return g_user_logged_in; }

const char *ra_get_username() { return g_username; }
const char *ra_get_token() { return g_token; }

void ra_logout_user() {
  g_user_logged_in = false;
  g_username = NULL;
  g_token = NULL;
  rc_client_logout(g_client);
}

void ra_login_user(const char *username, const char *password, void *userdata) {
  // This will generate an HTTP payload and call the server_call chain above.
  // Eventually, login_callback will be called to let us know if the login was
  // successful.
  g_user_logged_in = false;
  rc_client_begin_login_with_password(g_client, username, password,
                                      login_callback, userdata);
}

void ra_login_token_user(const char *username, const char *token,
                         void *userdata) {
  // This is exactly the same functionality as
  // rc_client_begin_login_with_password, but uses the token captured from the
  // first login instead of a password. Note that it uses the same callback.
  g_user_logged_in = false;
  rc_client_begin_login_with_token(g_client, username, token, login_callback,
                                   userdata);
}

static void load_game_callback(int result, const char *error_message,
                               rc_client_t *client, void *userdata) {
  if (result != RC_OK) {
    rc_client_set_hardcore_enabled(client, false);
    snprintf(load_game_error_message, sizeof(load_game_error_message),
             "RA load failed: %s", error_message);
    notify_load_game(userdata);
    return;
  } else {
    memset(load_game_error_message, 0, sizeof(load_game_error_message));
  }

  if (!rc_client_is_processing_required(client)) {
    rc_client_set_hardcore_enabled(client, false);
  }

  g_leaderboard_list = rc_client_create_leaderboard_list(
      client, RC_CLIENT_LEADERBOARD_LIST_GROUPING_NONE);

  g_game_loaded = true;
  notify_load_game(userdata);
}

void ra_welcome() {
  if (strlen(load_game_error_message) > 0) {
    rdp_onscreen_message(load_game_error_message, true);
  }
  if (!g_game_loaded)
    return;

  char buffer[512];

  const rc_client_game_t *game = rc_client_get_game_info(g_client);
  rc_client_user_game_summary_t summary;
  rc_client_get_user_game_summary(g_client, &summary);

  int message_length = snprintf(
      buffer, sizeof(buffer), "%s\nMode: %s\n", game->title,
      rc_client_get_hardcore_enabled(g_client) ? "Hardcore" : "Softcore");

  if (summary.num_core_achievements != 0) {
    snprintf(buffer + message_length, sizeof(buffer) - message_length,
             "%u/%u achievements unlocked", summary.num_unlocked_achievements,
             summary.num_core_achievements);
  } else {
    snprintf(buffer + message_length, sizeof(buffer) - message_length,
             "Game has no achievements");
  }
  rdp_onscreen_message(buffer, true);
}

void ra_load_game(const uint8_t *rom, size_t rom_size, void *userdata) {
  if (!g_user_logged_in) {
    notify_load_game(userdata);
    return;
  }

  rc_client_begin_identify_and_load_game(g_client, RC_CONSOLE_NINTENDO_64, NULL,
                                         rom, rom_size, load_game_callback,
                                         userdata);
}

void ra_set_dmem(const uint8_t *dmem, size_t dmem_size) {
  g_dmem = dmem;
  g_dmem_size = dmem_size;
}

static void leaderboard_submitted(const rc_client_leaderboard_t *leaderboard) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "Leaderboard submitted: %s - %s",
           leaderboard->title, leaderboard->tracker_value);
  rdp_onscreen_message(buffer, false);
}

static void achievement_triggered(const rc_client_achievement_t *achievement) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "Unlocked: %s", achievement->title);
  rdp_onscreen_message(buffer, false);
}

static void game_completed(rc_client_t *client) {
  char buffer[512];
  const rc_client_game_t *game = rc_client_get_game_info(client);

  snprintf(buffer, sizeof(buffer), "%s: %s",
           rc_client_get_hardcore_enabled(client) ? "Mastered" : "Completed",
           game->title);
  rdp_onscreen_message(buffer, false);
}

static void subset_completed(const rc_client_subset_t *subset,
                             rc_client_t *client) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "Subset %s: %s",
           rc_client_get_hardcore_enabled(client) ? "mastered" : "completed",
           subset->title);
  rdp_onscreen_message(buffer, false);
}

static void server_error(const rc_client_server_error_t *server_error) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "RA server error: %s",
           server_error->error_message);
  rdp_onscreen_message(buffer, false);
}

static const char *get_leaderboard_title(const char *display) {
  if (g_leaderboard_list == NULL)
    return NULL;

  for (uint32_t i = 0; i < g_leaderboard_list->num_buckets; i++) {
    for (uint32_t j = 0; j < g_leaderboard_list->buckets[i].num_leaderboards;
         j++) {
      // this looks like a mistake, but it is intentional.
      // Leaderboard trackers don't contain the title,
      // but we take advantage of the fact that tracker_value and display point
      // to the same char* to find a matching leaderboard.
      if (g_leaderboard_list->buckets[i].leaderboards[j]->tracker_value ==
          display) {
        return g_leaderboard_list->buckets[i].leaderboards[j]->title;
      }
    }
  }
  return NULL;
}

static void event_handler(const rc_client_event_t *event, rc_client_t *client) {
  switch (event->type) {
  case RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED:
    achievement_triggered(event->achievement);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_STARTED:
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_FAILED:
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_SUBMITTED:
    if (g_leaderboard)
      leaderboard_submitted(event->leaderboard);
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_CHALLENGE_INDICATOR_SHOW:
    if (g_challenge)
      achievement_challenge_indicator_add(event->achievement->title);
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_CHALLENGE_INDICATOR_HIDE:
    if (g_challenge)
      achievement_challenge_indicator_remove(event->achievement->title);
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_PROGRESS_INDICATOR_SHOW:
    achievement_progress_add(event->achievement->title,
                             event->achievement->measured_progress);
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_PROGRESS_INDICATOR_HIDE:
    achievement_progress_remove();
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_PROGRESS_INDICATOR_UPDATE:
    achievement_progress_add(event->achievement->title,
                             event->achievement->measured_progress);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_SHOW:
    if (g_leaderboard)
      leaderboard_tracker_add(
          event->leaderboard_tracker->id,
          get_leaderboard_title(event->leaderboard_tracker->display),
          event->leaderboard_tracker->display);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_HIDE:
    if (g_leaderboard)
      leaderboard_tracker_remove(event->leaderboard_tracker->id);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_UPDATE:
    if (g_leaderboard)
      leaderboard_tracker_add(
          event->leaderboard_tracker->id,
          get_leaderboard_title(event->leaderboard_tracker->display),
          event->leaderboard_tracker->display);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_SCOREBOARD:
    break;
  case RC_CLIENT_EVENT_GAME_COMPLETED:
    game_completed(client);
    break;
  case RC_CLIENT_EVENT_SUBSET_COMPLETED:
    subset_completed(event->subset, client);
    break;
  case RC_CLIENT_EVENT_SERVER_ERROR:
    server_error(event->server_error);
    break;
  default:
    printf("RetroAchievements: Unhandled event %d\n", event->type);
    break;
  }
}

void ra_init_client(bool hardcore, bool challenge, bool leaderboard) {
  // Create the client instance (using a global variable simplifies this
  // example)
  g_client = rc_client_create(read_memory, server_call);
  g_game_loaded = false;
  g_user_logged_in = false;
  g_dmem = NULL;
  g_dmem_size = 0;

  // Provide a logging function to simplify debugging
  rc_client_enable_logging(g_client, RC_CLIENT_LOG_LEVEL_WARN, log_message);

  rc_client_set_event_handler(g_client, event_handler);

  rc_client_set_hardcore_enabled(g_client, hardcore);

  g_challenge = challenge;
  g_leaderboard = leaderboard;
}

bool ra_get_hardcore() {
  if (!g_user_logged_in)
    return false;
  return rc_client_get_hardcore_enabled(g_client);
}

void ra_shutdown_client() {
  if (g_leaderboard_list) {
    rc_client_destroy_leaderboard_list(g_leaderboard_list);
    g_leaderboard_list = NULL;
  }
  if (g_client) {
    // Release resources associated to the client instance
    rc_client_destroy(g_client);
    g_client = NULL;
  }
}

void ra_do_frame() {
  if (!g_game_loaded)
    return;

  rc_client_do_frame(g_client);
}

void ra_do_idle() {
  if (!g_game_loaded)
    return;

  rc_client_idle(g_client);
}

size_t ra_state_size() {
  if (!g_game_loaded)
    return 0;

  return rc_client_progress_size(g_client);
}

void ra_save_state(uint8_t *state, size_t state_size) {
  if (!g_game_loaded)
    return;
  if (rc_client_serialize_progress_sized(g_client, state, state_size) !=
      RC_OK) {
    printf("RetroAchievements: Failed to serialize progress\n");
  }
}

void ra_load_state(const uint8_t *state, size_t state_size) {
  if (!g_game_loaded)
    return;
  if (rc_client_deserialize_progress_sized(g_client, state, state_size) !=
      RC_OK) {
    printf("RetroAchievements: Failed to deserialize progress\n");
  }
}

void ra_display_inprogress_achievements(void *userdata) {
  if (!g_game_loaded)
    return;

  rc_client_achievement_list_t *list = rc_client_create_achievement_list(
      g_client, RC_CLIENT_ACHIEVEMENT_CATEGORY_CORE,
      RC_CLIENT_ACHIEVEMENT_LIST_GROUPING_LOCK_STATE);

  char buffer[1024] = {0};
  size_t buffer_length = 0;

  for (uint32_t i = 0; i < list->num_buckets; i++) {
    if (list->buckets[i].bucket_type == RC_CLIENT_ACHIEVEMENT_BUCKET_LOCKED) {
      for (uint32_t j = 0; j < list->buckets[i].num_achievements; j++) {
        const rc_client_achievement_t *achievement =
            list->buckets[i].achievements[j];
        if (achievement->measured_percent > 0.0f) {
          if (buffer_length < sizeof(buffer) - 1) {
            buffer_length += snprintf(
                buffer + buffer_length, sizeof(buffer) - buffer_length,
                "%s: %s\n", achievement->title, achievement->measured_progress);
          }
        }
      }
    }
  }
  if (buffer_length > 0) {
    rdp_onscreen_message(buffer, false);
  }
  rc_client_destroy_achievement_list(list);
}
