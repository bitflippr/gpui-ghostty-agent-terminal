# Keyboard shortcuts

All actions below can be changed in **Settings > Keybindings**. Click a binding
and press its replacement. Escape cancels recording; Backspace restores the
default. Existing custom bindings are preserved when the application updates.

| Action | Windows / Linux | macOS |
| --- | --- | --- |
| Settings | Ctrl+, | Cmd+, |
| New Space | Ctrl+Shift+N | Cmd+Shift+N |
| New Tab | Ctrl+T | Cmd+T |
| Close Pane | Ctrl+Shift+W | Cmd+Shift+W |
| Split right / down | Ctrl+Shift+D / E | Cmd+Shift+D / E |
| Next / previous Tab | Ctrl+Tab / Ctrl+Shift+Tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Select Tab 1–8 / last | Alt+1–8 / 9 | Cmd+1–8 / 9 |
| Next / previous Space | Ctrl+Alt+PageDown / PageUp | Cmd+Option+PageDown / PageUp |
| Select Space 1–8 / last | Alt+Shift+1–8 / 9 | Cmd+Shift+1–8 / 9 |
| Close Tab | Ctrl+Alt+Shift+W | Cmd+Option+Shift+W |
| Close Space | Ctrl+Alt+W | Cmd+Option+W |
| Toggle sidebar | Ctrl+Shift+B | Cmd+Shift+B |
| Focus adjacent Pane | Ctrl+Alt+Arrow | Cmd+Option+Arrow |
| Next / previous Pane | Ctrl+Shift+] / [ | Cmd+Shift+] / [ |
| Resize split | Ctrl+Alt+Shift+Arrow | Cmd+Option+Shift+Arrow |
| Increase / decrease / reset font | Ctrl+= / - / 0 | Cmd+= / - / 0 |
| Scroll one line up / down | Ctrl+Shift+Up / Down | Cmd+Shift+Up / Down |
| Scroll one page up / down | Shift+PageUp / PageDown | Shift+PageUp / PageDown |
| Scroll to top / bottom | Shift+Home / End | Shift+Home / End |
| Move Pane | Ctrl+Shift+M | Cmd+Shift+M |
| Next agent | Ctrl+Shift+U | Cmd+Shift+U |
| Previous agent | Ctrl+Alt+Shift+U | Cmd+Option+Shift+U |
| Expand / collapse selected Space's agent list | Ctrl+Shift+. | Cmd+Shift+. |
| Quit Application | Ctrl+Shift+Q | Cmd+Q |

The font-increase default also accepts Ctrl++ (Cmd++ on macOS). Assigning a
custom font-increase binding removes this alias. An explicit binding to the
plus chord takes priority over the alias.

Tab, Space, Pane, and agent cycling wrap at the ends. Agent navigation visits
recognized agent terminals across all Spaces in layout order. Directional Pane
focus uses the rendered layout and stays within the selected Tab. Resizing moves
the nearest matching split divider by one terminal cell, within the existing
layout limits.

Move Pane opens a destination picker covering all Spaces. Use arrows or Tab
(Shift+Tab for backwards) to choose, Enter to move to the right of the destination
Pane, or Escape to cancel. The existing mouse destination selection also works.
The Terminal Session continues running when its Pane moves.

Scrollback shortcuts operate on terminal history without sending keys or mouse
reports to the running program. They do not scroll an alternate-screen program's
own view; use that program's controls for that.

Close Pane, Close Tab, and Close Space retain the existing busy-terminal
confirmation. **Quit Application** ends all Terminal Sessions. **Close Window**
retains the Application and its Terminal Sessions when desktop presence is
available.
