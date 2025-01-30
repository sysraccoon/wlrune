# wlrune

Mouse gestures utility for **wayland compositors**

## Installation

**nix**

```sh
# imperative installation
nix profile install github:sysraccoon/wlrune
```

**cargo**

```sh
# make sure you have pkg-config, libxkbcommon and wayland packages
cargo install --git https://github.com/sysraccoon/mouse-gestures.git
```

## Usage

Record some patterns (any cursor movements will be recorded, then click the 
mouse button to end the recording):

```sh
wlrune record --name left
wlrune record --name right
# ...
```

Create config file:
```sh
mkdir ~/.config/wlrune
touch ~/.config/wlrune/config.yaml
```

Basic config

```yaml
# This section is optional
recognizer:
    # The percentage of similarity between the original pattern
    # and the user input requiret to trigger the command
    command_execute_treshold: 0.8
    # Point count required to trigger command or save new pattern
    point_count_treshold: 10
    # Acceptable range for pattern rotation (degrees)
    rotation_angle_range: 10.0
    # Acceptable accuracy in pattern rotation (degrees)
    rotation_angle_treshold: 2.0
    # The number of points to which the pattern is reduced fo recognition
    resample_num_points: 64
    # Width used for recognition (may not match screen size)
    width: 100.0
    # Height used for recognition (may not match screen size)
    height: 100.0

# Record patterns by using `wlrune record --name up` and define commands below
commands:
  - pattern: "up"
    command: "firefox"
  - pattern: "down"
    command: "kitty"
```

Start recognition:
```sh
wlrune recognize
```

Keybinding for `wlrune recognize` is compositor specific. You can search mouse 
button code by using `wev` and use code as present bellow.

**sway**

`~/.config/sway/config`:

```
bindcode --whole-window --no-repeat 276 exec wlrune recognize
```

**hyprland**

`~/.config/hypr/hyprland.conf`:
```
bind = , code:276, exec, wlrune recognize
```

