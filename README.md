# Rust_Iced_Draw
Drawing program using Iced-rs

https://github.com/user-attachments/assets/01f8fb41-082a-45b5-92a6-806814385578

## TODO Items:
* Text widget needs an edit and rotation mode
* Text widget cursor blink may vary according to screen resolution and maybe os,  
   adjust in helpers.rs get_blink_position by using a factor for the x position.
* Text widget needs it's own cache
* Svg image widget
* turn degrees display on or off
* add a pixels amount when drawing or a scaled amount like inches mm, etc.

## Updates to Main since v0.1.0
* Fixed elipse top positioning during new
* Fixed circle points not showing during new
* TODO items in Readme

## Instructions:

Select a geometry using the radio buttons.

If a polyline or polygon is selected, also enter the numbers of sides.

The freehand and polyline are similar.  
The freehand will continue until the enter key is pressed.
The polyline will end when the number of points are reached.

Colors can be selected using the Draw Color and Canvas Color.

Geometries can be save and loaded via the corresponding buttons.
They are stored under the resource folder in a json file.

The width of the curves can be changed by the width input.

### How to Draw:

Drawing is done by clicking the left mouse button while its in the canvas area.
Holding the mouse down while drawing doesn't do anything, just click and move mouse.
The points are set by each button click.
Different geometries have a different number of clicks before finished.

* Arc - 3 clicks - best to move left after the first click
* Bezier - 3 clicks
* Circle - 2 clicks
* Ellipse 2 clicks, best to move left after the first one
* Line - 2 clicks
* PolyLine - based on the poly points entered
* Polygon - 2 clicks, poly points determe the sides only
* RightTriangle - 3 clicks
* Text - 1 click then start typing, another click to end
* FreeHand  - unlimited clicks, press enter to end.


The curves can be edited by selecting the Edit mode and clicking
near the curve you want to edit.  The edit search is based on the 
midpoint of the curve except for the freehand which is based on the first point.

Next move the mouse close to the point of interest and click again.
Position the point where needed and click once more to finish.
if you selected the mid point, you can drag the curve to a new place.

The curves can be rotated in two ways.
1. if in edit mode, one of the points will rotate the curve.
2. mouse scrolling.



## Program flow:

### Overview: 
The key program flow areas are the DrawPending and DrawCurve implements

The flow in the DrawPending is used to update the curves until they are complete.
The exception is the text widget, discussed later.

In a normal operation, the draw_all() method draws the curves from cache.  
A redraw doesn't happen until the cache is cleared.  So if a pending curve is present,
the pending curve is drawn only leaving the other curves displayed.
When the pending curve is finished, it's added to the curves and the cache is cleared,
which causes a refresh of the canvas.

If a curve is in the edit or rotation mode, the cache is cleared and redrawn skipping
over the curve that's being edited.  The Pending::Edit or Pending::Rotation curve is then 
displayed.

Once the mouse is clicked, a canvas event for the mouse left button pressed occurs.
Pending at this point is None, so parameters are added to the Some(Pending).

During the next mouse clicks, depending on the draw method, New, Edit, Rotate,
the Pending is matched.

### Pending flow:

For Pending::New, the Pening returns itself until a widget criteria is met and 
then the curve is returned.

For Pending::Edit, 3 pending happen.
1st click => Pending::Edit is None, so the closest widget is found and a Pending::EditSecond
is passed on.
2nd click => The closest point to be editied is found and a Pending::EditThird is passed on.
3rd click => The edited curve is returned to have the curves updated and displayed.

For Pending::Rotation, the mouse scroll event is used to rotate the widgets.
1st click => closest widget is found and highlighted.
mouse scroll => widget is rotated
2nd click => rotation ends and curve return for updating.

The Text widget is different because of need to have a blinking cursor.
Canvas, at this time, does not have a timed event so the main subscription event is used.
The subscription event is turned on when a Text widget is selected.  At each tick,
the canvas cache is cleared resulting in a redraw.  Instead of returning pending curve, 
Pending returns a curve to the main for display.  Currently, the Text widget is added to
the curves hashmap but a later update will have the Text widgets in their own cache to improve 
performance.
