#include "retroachievements.h"
#include "../parallel-rdp/interface.hpp"
#include <rc_client.h>
#include <rc_consoles.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void rust_server_call(const char *url, const char *post_data,
                      rc_client_server_callback_t callback,
                      void *callback_data);
void store_retroachievements_credentials(const char *username,
                                         const char *token, void *userdata);

rc_client_t *g_client = NULL;
const uint8_t *g_dmem = NULL;
size_t g_dmem_size = 0;
bool g_game_loaded = false;
bool g_user_logged_in = false;
const char *g_username = NULL;
const char *g_token = NULL;

static uint32_t read_memory(uint32_t address, uint8_t *buffer,
                            uint32_t num_bytes, rc_client_t *client) {
  if (address + num_bytes >= g_dmem_size)
    return 0;
  memcpy(buffer, &g_dmem[address], num_bytes);
  return num_bytes;
}

static void server_call(const rc_api_request_t *request,
                        rc_client_server_callback_t callback,
                        void *callback_data, rc_client_t *client) {
  rust_server_call(request->url, request->post_data, callback, callback_data);
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
    store_retroachievements_credentials(NULL, NULL, userdata);
    return;
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
  char buffer[512];
  if (result != RC_OK) {
    snprintf(buffer, sizeof(buffer), "RA load failed: %s", error_message);
    rdp_onscreen_message(buffer);
    rdp_onscreen_message(buffer); // show it a bit longer
    return;
  }

  const rc_client_game_t *game = rc_client_get_game_info(client);
  rc_client_user_game_summary_t summary;
  rc_client_get_user_game_summary(client, &summary);

  int hardcore_enabled = rc_client_get_hardcore_enabled(client);
  int message_length =
      snprintf(buffer, sizeof(buffer), "RA loaded: %s\nMode: %s\n", game->title,
               hardcore_enabled ? "Hardcore" : "Softcore");

  if (summary.num_core_achievements != 0) {
    snprintf(buffer + message_length, sizeof(buffer) - message_length,
             "%u/%u achievements unlocked", summary.num_unlocked_achievements,
             summary.num_core_achievements);
  } else {
    snprintf(buffer + message_length, sizeof(buffer) - message_length,
             "Game has no achievements");
  }
  rdp_onscreen_message(buffer);
  rdp_onscreen_message(buffer); // show it a bit longer

  g_game_loaded = true;
}

void ra_load_game(const uint8_t *rom, size_t rom_size) {
  if (!g_user_logged_in)
    return;

  rc_client_begin_identify_and_load_game(g_client, RC_CONSOLE_NINTENDO_64, NULL,
                                         rom, rom_size, load_game_callback,
                                         NULL);
}

void ra_set_dmem(const uint8_t *dmem, size_t dmem_size) {
  g_dmem = dmem;
  g_dmem_size = dmem_size;
}

static void leaderboard_started(const rc_client_leaderboard_t *leaderboard) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "RA leaderboard attempt started: %s",
           leaderboard->title);
  rdp_onscreen_message(buffer);
}

static void leaderboard_failed(const rc_client_leaderboard_t *leaderboard) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "RA leaderboard attempt failed: %s",
           leaderboard->title);
  rdp_onscreen_message(buffer);
}

static void leaderboard_submitted(const rc_client_leaderboard_t *leaderboard) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "RA leaderboard submitted: %s - %s",
           leaderboard->title, leaderboard->tracker_value);
  rdp_onscreen_message(buffer);
}

static void achievement_triggered(const rc_client_achievement_t *achievement) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "RA unlocked: %s", achievement->title);
  rdp_onscreen_message(buffer);
}

static void
achievement_progress_updated(const rc_client_achievement_t *achievement) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "RA updated: %s: %s", achievement->title,
           achievement->measured_progress);
  rdp_onscreen_message(buffer);
}

static void game_completed(rc_client_t *client) {
  char buffer[512];
  const rc_client_game_t *game = rc_client_get_game_info(client);

  snprintf(buffer, sizeof(buffer), "RA %s: %s",
           rc_client_get_hardcore_enabled(client) ? "mastered" : "completed",
           game->title);
  rdp_onscreen_message(buffer);
}

static void subset_completed(const rc_client_subset_t *subset,
                             rc_client_t *client) {
  char buffer[512];

  snprintf(buffer, sizeof(buffer), "RA subset %s: %s",
           rc_client_get_hardcore_enabled(client) ? "mastered" : "completed",
           subset->title);
  rdp_onscreen_message(buffer);
}

static void event_handler(const rc_client_event_t *event, rc_client_t *client) {
  switch (event->type) {
  case RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED:
    achievement_triggered(event->achievement);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_STARTED:
    leaderboard_started(event->leaderboard);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_FAILED:
    leaderboard_failed(event->leaderboard);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_SUBMITTED:
    leaderboard_submitted(event->leaderboard);
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_CHALLENGE_INDICATOR_SHOW:
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_CHALLENGE_INDICATOR_HIDE:
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_PROGRESS_INDICATOR_SHOW:
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_PROGRESS_INDICATOR_HIDE:
    break;
  case RC_CLIENT_EVENT_ACHIEVEMENT_PROGRESS_INDICATOR_UPDATE:
    achievement_progress_updated(event->achievement);
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_SHOW:
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_HIDE:
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_UPDATE:
    break;
  case RC_CLIENT_EVENT_LEADERBOARD_SCOREBOARD:
    break;
  case RC_CLIENT_EVENT_GAME_COMPLETED:
    game_completed(client);
    break;
  case RC_CLIENT_EVENT_SUBSET_COMPLETED:
    subset_completed(event->subset, client);
    break;
  default:
    printf("RetroAchievements: Unhandled event %d\n", event->type);
    break;
  }
}

void ra_init_client(bool hardcore) {
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
}

bool ra_get_hardcore() {
  if (!g_user_logged_in)
    return false;
  return rc_client_get_hardcore_enabled(g_client);
}

void ra_shutdown_client() {
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
