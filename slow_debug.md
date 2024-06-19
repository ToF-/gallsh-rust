O: when option -e  (show extract palette) is set, on the first time the pictures appears, palette doesn't show. After next page or prev page, palette is shown.

H: the first call to `show_grid` before any keyboard event, doesn't have all the same decision flags to show the palette. 

A: have `show_grid`, trace the state of all its objects, then call the program and compare trace log before first keyb event and after next_page evt
O: there's a difference in the state of some variables
```
sort pictures by Random
1730 entries
repository.palette_extract_on() true
repository.index_from_position((col,row)).is_some() true
drawing_area.is_visible() false
picture.is_visible() false
picture.width() 0
vbox.allocation().height() 0
move to next page
repository.palette_extract_on() true
repository.index_from_position((col,row)).is_some() true
drawing_area.is_visible() true
picture.is_visible() true
picture.width() 1250
vbox.allocation().height() 1009
quit gallery show

```
O: `picture` is not visible when entering `show_grid` the first time.
O: palette drawing is within an if `picture.is_visible()`
A: remove this if (making the palette drawing unconditionnal)
O: same symptom, and `picture` is still not set to visible
H: picture is made visible after setting the picture file, but not the drawing area for the palette
A: trace `picture.is_visible and drawing_area.is_visible` just before the drawing area (palette) block
```
picture.is_visible() false
drawing_area.is_visible() false
move to next page
picture.is_visible() true
drawing_area.is_visible() true
```
Q: why is `picture.is_visible` false when I can see the picture in the app ?
A: look up for is_visble function on gtk documentation
O: nothing much on special conditions or sequence of visibility
A: introduce a (keyboard) pause to see what is visible at the time of printing the trace
O: before any showing happend, values are given by `is_visible` is not reliable
A: have the program execute `window.present()`  before the first `show_grid()`
O: doesn't change the behaviour
A: remove the first `show_grid` at the end of `connect_activate` to see what happens
O: the first picture is not shown, then a first key movement will show the picture but not the palette, then a next key movement show both
...

H: the `draw_palette` inside the palette object is not called during preparation of palette
A: call `show` after preparation of the palette
O: no change in symptom
Q: instead of having the keyboard movement drive the display, should I attach the display code to a `paint` event of some sort?
A: look up info on show / paint event in gtk
... no internet connection
A: a jump to the first page provokes the correct display
O: no change
A: a trace of the end of `draw_palette` method
O: on first show, the draw_palette is not called
A: call a `vbox.queue_draw();` at the end of setting the drawing area
O: not called (because it's not visible)
A: put a trace earlier
O: the two object are not visible the first time `show_grid` is called
