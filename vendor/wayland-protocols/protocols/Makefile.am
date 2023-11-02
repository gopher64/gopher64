ACLOCAL_AMFLAGS = -I m4

unstable_protocols =								\
	unstable/pointer-gestures/pointer-gestures-unstable-v1.xml		\
	unstable/fullscreen-shell/fullscreen-shell-unstable-v1.xml		\
	unstable/linux-dmabuf/linux-dmabuf-unstable-v1.xml			\
	unstable/text-input/text-input-unstable-v1.xml				\
	unstable/text-input/text-input-unstable-v3.xml				\
	unstable/input-method/input-method-unstable-v1.xml			\
	unstable/xdg-shell/xdg-shell-unstable-v5.xml				\
	unstable/xdg-shell/xdg-shell-unstable-v6.xml				\
	unstable/relative-pointer/relative-pointer-unstable-v1.xml		\
	unstable/pointer-constraints/pointer-constraints-unstable-v1.xml	\
	unstable/tablet/tablet-unstable-v1.xml			                \
	unstable/tablet/tablet-unstable-v2.xml			                \
	unstable/xdg-foreign/xdg-foreign-unstable-v1.xml			\
	unstable/xdg-foreign/xdg-foreign-unstable-v2.xml			\
	unstable/idle-inhibit/idle-inhibit-unstable-v1.xml			\
	unstable/xwayland-keyboard-grab/xwayland-keyboard-grab-unstable-v1.xml	\
	unstable/keyboard-shortcuts-inhibit/keyboard-shortcuts-inhibit-unstable-v1.xml \
	unstable/xdg-output/xdg-output-unstable-v1.xml				\
	unstable/input-timestamps/input-timestamps-unstable-v1.xml	\
	unstable/xdg-decoration/xdg-decoration-unstable-v1.xml	\
	unstable/linux-explicit-synchronization/linux-explicit-synchronization-unstable-v1.xml \
	unstable/primary-selection/primary-selection-unstable-v1.xml		\
	$(NULL)

stable_protocols =								\
	stable/presentation-time/presentation-time.xml				\
	stable/viewporter/viewporter.xml					\
	stable/xdg-shell/xdg-shell.xml						\
	$(NULL)

staging_protocols =								\
	staging/xdg-activation/xdg-activation-v1.xml				\
	$(NULL)

misc_documentation =								\
	staging/xdg-activation/x11-interoperation.rst				\
	$(NULL)

nobase_dist_pkgdata_DATA =							\
	$(unstable_protocols)							\
	$(stable_protocols)							\
	$(staging_protocols)							\
	$(NULL)

dist_noinst_DATA =								\
	$(sort $(foreach p,$(unstable_protocols),$(dir $p)README))		\
	$(sort $(foreach p,$(stable_protocols),$(dir $p)README))		\
	$(sort $(foreach p,$(staging_protocols),$(dir $p)README))		\
	$(misc_documentation)							\
	README.md								\
	GOVERNANCE.md								\
	MEMBERS.md								\
	meson.build								\
	meson_options.txt							\
	tests/meson.build							\
	tests/build-cxx.cc.in							\
	tests/build-pedantic.c.in						\
	tests/replace.py							\
	tests/scan.sh								\
	$(NULL)

noarch_pkgconfig_DATA = wayland-protocols.pc

dist_check_SCRIPTS = tests/scan.sh

TESTS = $(unstable_protocols) $(stable_protocols) $(staging_protocols)
TEST_EXTENSIONS = .xml
AM_TESTS_ENVIRONMENT = SCANNER='$(wayland_scanner)'; export SCANNER;
XML_LOG_COMPILER = $(srcdir)/tests/scan.sh
