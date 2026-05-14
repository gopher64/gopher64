#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

void ra_init_client(bool hardcore, bool challenge, bool leaderboard);
void ra_shutdown_client();
void ra_welcome();
bool ra_get_hardcore();
void ra_load_game(const uint8_t *rom, size_t rom_size, void *userdata);
void ra_unload_game();
void ra_set_rdram(const uint8_t *rdram, size_t rdram_size);
void ra_do_frame();
void ra_do_idle();
void ra_http_callback(const char *content, size_t content_size, int status_code,
                      void *callback, void *callback_data);
void ra_logout_user();
void ra_login_user(const char *username, const char *password, void *userdata);
void ra_login_token_user(const char *username, const char *token,
                         void *userdata);
const char *ra_get_username();
const char *ra_get_token();
size_t ra_state_size();
void ra_save_state(uint8_t *state, size_t state_size);
void ra_load_state(const uint8_t *state, size_t state_size);

const char *ra_get_rich_presence();
void ra_get_game_info(const char **game_title, const char **game_image_url);

#ifdef __cplusplus
extern "C" {
#endif
void ra_display_inprogress_achievements(void *userdata);
#ifdef __cplusplus
}
#endif
