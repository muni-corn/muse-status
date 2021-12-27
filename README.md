# muse-status

Shamelessly the most beautiful status command for your Linux
setup.

`muse-status` can provide output for the following formats:

-	`lemonbar`

-	`i3bar`/`swaybar`

-	Pango markup

# Building

The following dependencies are required to build `muse-status` and
its daemon:

-	pkg-config

-	alsa-lib (development files)

-	dbus (development files)

-	openssl (development files)

# Running

## The `volume` module

requires either `pamixer` or `amixer` to be in your `$PATH`

## The `network` module

requires `ping` and `ip` to be in your `$PATH`

## The `mpris` module

requires `dbus`
