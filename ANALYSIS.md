# p5.js and Raylib Analysis for wasm96

This document analyzes the methods from p5.js and Raylib to determine equivalents in wasm96, what makes sense to implement next, and what is JS/Web-specific and thus not applicable.

## p5.js Methods Analysis

### Shape

#### 2D Primitives
- arc() - Not implemented. Makes sense to implement (draw arc).
- circle() - Equivalent: GRAPHICS_CIRCLE
- ellipse() - Not implemented. Makes sense to implement (general ellipse).
- line() - Equivalent: GRAPHICS_LINE
- point() - Equivalent: GRAPHICS_POINT
- quad() - Not implemented. Makes sense to implement (quadrilateral).
- rect() - Equivalent: GRAPHICS_RECT
- square() - Equivalent: GRAPHICS_RECT (special case)
- triangle() - Equivalent: GRAPHICS_TRIANGLE

#### 3D Primitives
- createModel() - Not implemented. Makes sense to implement (load 3D model).
- loadModel() - Not implemented. Makes sense to implement.
- model() - Not implemented. Makes sense to implement (draw 3D model).
- beginGeometry() - Not implemented. Makes sense to implement (start custom geometry).
- box() - Not implemented. Makes sense to implement (3D box).
- buildGeometry() - Not implemented. Makes sense to implement.
- cone() - Not implemented. Makes sense to implement.
- cylinder() - Not implemented. Makes sense to implement.
- ellipsoid() - Not implemented. Makes sense to implement.
- endGeometry() - Not implemented. Makes sense to implement.
- freeGeometry() - Not implemented. Makes sense to implement.
- plane() - Not implemented. Makes sense to implement.
- sphere() - Not implemented. Makes sense to implement.
- torus() - Not implemented. Makes sense to implement.

#### Attributes
- ellipseMode() - Not implemented. Makes sense to implement (ellipse drawing mode).
- noSmooth() - Not implemented. Makes sense to implement (disable antialiasing).
- rectMode() - Not implemented. Makes sense to implement (rectangle drawing mode).
- smooth() - Not implemented. Makes sense to implement (enable antialiasing).
- strokeCap() - Not implemented. Makes sense to implement (line cap style).
- strokeJoin() - Not implemented. Makes sense to implement (line join style).
- strokeWeight() - Not implemented. Makes sense to implement (line width).

#### Curves
- bezier() - Partial equivalent: GRAPHICS_BEZIER_CUBIC. Full makes sense to implement.
- bezierDetail() - Not implemented. Makes sense to implement.
- bezierPoint() - Not implemented. Makes sense to implement.
- bezierTangent() - Not implemented. Makes sense to implement.
- curve() - Not implemented. Makes sense to implement (Catmull-Rom spline).
- curveDetail() - Not implemented. Makes sense to implement.
- curvePoint() - Not implemented. Makes sense to implement.
- curveTangent() - Not implemented. Makes sense to implement.
- curveTightness() - Not implemented. Makes sense to implement.

#### Vertex
- beginContour() - Not implemented. Makes sense to implement (hole in shape).
- beginShape() - Not implemented. Makes sense to implement (start custom shape).
- bezierVertex() - Not implemented. Makes sense to implement.
- curveVertex() - Not implemented. Makes sense to implement.
- endContour() - Not implemented. Makes sense to implement.
- endShape() - Not implemented. Makes sense to implement.
- normal() - Not implemented. Makes sense to implement (3D normal).
- quadraticVertex() - Not implemented. Makes sense to implement.
- vertex() - Not implemented. Makes sense to implement.

### Color

#### Creating & Reading
- alpha() - Not implemented. Makes sense to implement.
- blue() - Not implemented. Makes sense to implement.
- brightness() - Not implemented. Makes sense to implement.
- color() - Not implemented. Makes sense to implement.
- green() - Not implemented. Makes sense to implement.
- hue() - Not implemented. Makes sense to implement.
- lerpColor() - Not implemented. Makes sense to implement.
- lightness() - Not implemented. Makes sense to implement.
- paletteLerp() - Not implemented. Makes sense to implement.
- red() - Not implemented. Makes sense to implement.
- saturation() - Not implemented. Makes sense to implement.

#### Setting
- background() - Equivalent: GRAPHICS_BACKGROUND
- beginClip() - Not implemented. Makes sense to implement (clipping).
- clear() - Not implemented. Makes sense to implement.
- clip() - Not implemented. Makes sense to implement.
- colorMode() - Not implemented. Makes sense to implement.
- endClip() - Not implemented. Makes sense to implement.
- erase() - Not implemented. Makes sense to implement.
- fill() - Not implemented. Makes sense to implement (fill color).
- noErase() - Not implemented. Makes sense to implement.
- noFill() - Not implemented. Makes sense to implement.
- noStroke() - Not implemented. Makes sense to implement.
- stroke() - Not implemented. Makes sense to implement (stroke color).

#### p5.Color
- setAlpha() - Not implemented. Makes sense to implement.
- setBlue() - Not implemented. Makes sense to implement.
- setGreen() - Not implemented. Makes sense to implement.
- setRed() - Not implemented. Makes sense to implement.
- toString() - Not implemented. Makes sense to implement.

### Typography

#### Attributes
- textAlign() - Not implemented. Makes sense to implement.
- textAscent() - Not implemented. Makes sense to implement.
- textDescent() - Not implemented. Makes sense to implement.
- textLeading() - Not implemented. Makes sense to implement.
- textSize() - Not implemented. Makes sense to implement.
- textStyle() - Not implemented. Makes sense to implement.

#### Loading & Displaying
- loadFont() - Equivalent: GRAPHICS_FONT_REGISTER_*
- text() - Equivalent: GRAPHICS_TEXT_KEY
- textFont() - Not implemented. Makes sense to implement.
- textWidth() - Equivalent: GRAPHICS_TEXT_MEASURE_KEY
- textWrap() - Not implemented. Makes sense to implement.

#### p5.Font
- font - Not implemented. Makes sense to implement.
- textBounds() - Not implemented. Makes sense to implement.
- textToPoints() - Not implemented. Makes sense to implement.

### Image

- createImage() - Not implemented. Makes sense to implement.
- saveCanvas() - JS/Web-specific. No.
- saveFrames() - JS/Web-specific. No.
- image() - Equivalent: GRAPHICS_PNG_DRAW_KEY etc.
- imageMode() - Not implemented. Makes sense to implement.
- loadImage() - Equivalent: GRAPHICS_PNG_REGISTER etc.
- noTint() - Not implemented. Makes sense to implement.
- tint() - Not implemented. Makes sense to implement.
- blend() - Not implemented. Makes sense to implement.
- copy() - Not implemented. Makes sense to implement.
- filter() - Not implemented. Makes sense to implement.
- get() - Not implemented. Makes sense to implement.
- loadPixels() - Not implemented. Makes sense to implement.
- pixels - Not implemented. Makes sense to implement.
- set() - Not implemented. Makes sense to implement.
- updatePixels() - Not implemented. Makes sense to implement.

#### p5.Image
- blend() - Not implemented. Makes sense to implement.
- copy() - Not implemented. Makes sense to implement.
- delay() - Not implemented. Makes sense to implement.
- filter() - Not implemented. Makes sense to implement.
- get() - Not implemented. Makes sense to implement.
- getCurrentFrame() - Not implemented. Makes sense to implement.
- height - Not implemented. Makes sense to implement.
- loadPixels() - Not implemented. Makes sense to implement.
- mask() - Not implemented. Makes sense to implement.
- numFrames() - Not implemented. Makes sense to implement.
- pause() - Not implemented. Makes sense to implement.
- pixelDensity() - Not implemented. Makes sense to implement.
- pixels - Not implemented. Makes sense to implement.
- play() - Not implemented. Makes sense to implement.
- reset() - Not implemented. Makes sense to implement.
- resize() - Not implemented. Makes sense to implement.
- save() - JS/Web-specific. No.
- set() - Not implemented. Makes sense to implement.
- setFrame() - Not implemented. Makes sense to implement.
- updatePixels() - Not implemented. Makes sense to implement.
- width - Not implemented. Makes sense to implement.

### Transform

- applyMatrix() - Not implemented. Makes sense to implement.
- resetMatrix() - Not implemented. Makes sense to implement.
- rotate() - Not implemented. Makes sense to implement (2D rotate).
- rotateX() - Not implemented. Makes sense to implement (3D).
- rotateY() - Not implemented. Makes sense to implement (3D).
- rotateZ() - Not implemented. Makes sense to implement (3D).
- scale() - Not implemented. Makes sense to implement.
- shearX() - Not implemented. Makes sense to implement.
- shearY() - Not implemented. Makes sense to implement.
- translate() - Not implemented. Makes sense to implement.

### Environment

- cursor() - JS/Web-specific. No.
- deltaTime - Not implemented. Makes sense to implement.
- describe() - JS/Web-specific. No.
- describeElement() - JS/Web-specific. No.
- displayDensity - JS/Web-specific. No.
- displayHeight - JS/Web-specific. No.
- displayWidth - JS/Web-specific. No.
- focused - JS/Web-specific. No.
- frameCount - Not implemented. Makes sense to implement.
- frameRate() - Not implemented. Makes sense to implement.
- fullscreen() - JS/Web-specific. No.
- getTargetFrameRate() - Not implemented. Makes sense to implement.
- getURL() - JS/Web-specific. No.
- getURLParams() - JS/Web-specific. No.
- getURLPath() - JS/Web-specific. No.
- gridOutput() - JS/Web-specific. No.
- height - Equivalent: GRAPHICS_SET_SIZE
- noCursor() - JS/Web-specific. No.
- pixelDensity() - Not implemented. Makes sense to implement.
- print() - JS/Web-specific. No.
- textOutput() - JS/Web-specific. No.
- webglVersion - JS/Web-specific. No.
- width - Equivalent: GRAPHICS_SET_SIZE
- windowHeight - JS/Web-specific. No.
- windowResized() - JS/Web-specific. No.
- windowWidth - JS/Web-specific. No.

### 3D

#### Camera
- camera() - Equivalent: GRAPHICS_CAMERA_LOOK_AT
- createCamera() - Not implemented. Makes sense to implement.
- frustum() - Not implemented. Makes sense to implement.
- linePerspective() - Not implemented. Makes sense to implement.
- noDebugMode() - Not implemented. Makes sense to implement.
- orbitControl() - Not implemented. Makes sense to implement.
- ortho() - Not implemented. Makes sense to implement.
- perspective() - Equivalent: GRAPHICS_CAMERA_PERSPECTIVE
- setCamera() - Not implemented. Makes sense to implement.

#### Interaction
- debugMode() - Not implemented. Makes sense to implement.

#### Lights
- ambientLight() - Not implemented. Makes sense to implement.
- directionalLight() - Not implemented. Makes sense to implement.
- imageLight() - Not implemented. Makes sense to implement.
- lightFalloff() - Not implemented. Makes sense to implement.
- lights() - Not implemented. Makes sense to implement.
- noLights() - Not implemented. Makes sense to implement.
- panorama() - Not implemented. Makes sense to implement.
- pointLight() - Not implemented. Makes sense to implement.
- specularColor() - Not implemented. Makes sense to implement.
- spotLight() - Not implemented. Makes sense to implement.

#### Material
- ambientMaterial() - Not implemented. Makes sense to implement.
- baseColorShader() - Not implemented. Makes sense to implement.
- baseMaterialShader() - Not implemented. Makes sense to implement.
- baseNormalShader() - Not implemented. Makes sense to implement.
- baseStrokeShader() - Not implemented. Makes sense to implement.
- createFilterShader() - Not implemented. Makes sense to implement.
- createShader() - Not implemented. Makes sense to implement.
- emissiveMaterial() - Not implemented. Makes sense to implement.
- loadShader() - Not implemented. Makes sense to implement.
- metalness() - Not implemented. Makes sense to implement.
- normalMaterial() - Not implemented. Makes sense to implement.
- resetShader() - Not implemented. Makes sense to implement.
- shader() - Not implemented. Makes sense to implement.
- shininess() - Not implemented. Makes sense to implement.
- specularMaterial() - Not implemented. Makes sense to implement.
- texture() - Not implemented. Makes sense to implement.
- textureMode() - Not implemented. Makes sense to implement.
- textureWrap() - Not implemented. Makes sense to implement.

#### p5.Camera
- camera() - Equivalent: GRAPHICS_CAMERA_LOOK_AT
- centerX - Not implemented. Makes sense to implement.
- centerY - Not implemented. Makes sense to implement.
- centerZ - Not implemented. Makes sense to implement.
- eyeX - Not implemented. Makes sense to implement.
- eyeY - Not implemented. Makes sense to implement.
- eyeZ - Not implemented. Makes sense to implement.
- frustum() - Not implemented. Makes sense to implement.
- lookAt() - Equivalent: GRAPHICS_CAMERA_LOOK_AT
- move() - Not implemented. Makes sense to implement.
- ortho() - Not implemented. Makes sense to implement.
- pan() - Not implemented. Makes sense to implement.
- perspective() - Equivalent: GRAPHICS_CAMERA_PERSPECTIVE
- roll() - Not implemented. Makes sense to implement.
- set() - Not implemented. Makes sense to implement.
- setPosition() - Not implemented. Makes sense to implement.
- slerp() - Not implemented. Makes sense to implement.
- tilt() - Not implemented. Makes sense to implement.
- upX - Not implemented. Makes sense to implement.
- upY - Not implemented. Makes sense to implement.
- upZ - Not implemented. Makes sense to implement.

#### p5.Shader
- copyToContext() - Not implemented. Makes sense to implement.
- inspectHooks() - Not implemented. Makes sense to implement.
- modify() - Not implemented. Makes sense to implement.
- setUniform() - Not implemented. Makes sense to implement.

### Rendering

- blendMode() - Not implemented. Makes sense to implement.
- clearDepth() - Not implemented. Makes sense to implement.
- createCanvas() - Equivalent: GRAPHICS_SET_SIZE
- createFramebuffer() - Not implemented. Makes sense to implement.
- createGraphics() - Not implemented. Makes sense to implement.
- drawingContext - JS/Web-specific. No.
- noCanvas() - JS/Web-specific. No.
- resizeCanvas() - Not implemented. Makes sense to implement.
- setAttributes() - Not implemented. Makes sense to implement.

#### p5.Framebuffer
- autoSized() - Not implemented. Makes sense to implement.
- begin() - Not implemented. Makes sense to implement.
- color - Not implemented. Makes sense to implement.
- createCamera() - Not implemented. Makes sense to implement.
- depth - Not implemented. Makes sense to implement.
- draw() - Not implemented. Makes sense to implement.
- end() - Not implemented. Makes sense to implement.
- get() - Not implemented. Makes sense to implement.
- loadPixels() - Not implemented. Makes sense to implement.
- pixelDensity() - Not implemented. Makes sense to implement.
- pixels - Not implemented. Makes sense to implement.
- remove() - Not implemented. Makes sense to implement.
- resize() - Not implemented. Makes sense to implement.
- updatePixels() - Not implemented. Makes sense to implement.

#### p5.Graphics
- createFramebuffer() - Not implemented. Makes sense to implement.
- remove() - Not implemented. Makes sense to implement.
- reset() - Not implemented. Makes sense to implement.

### Math

#### Calculation
- abs() - Not implemented. Makes sense to implement.
- ceil() - Not implemented. Makes sense to implement.
- constrain() - Not implemented. Makes sense to implement.
- dist() - Not implemented. Makes sense to implement.
- exp() - Not implemented. Makes sense to implement.
- floor() - Not implemented. Makes sense to implement.
- fract() - Not implemented. Makes sense to implement.
- lerp() - Not implemented. Makes sense to implement.
- log() - Not implemented. Makes sense to implement.
- mag() - Not implemented. Makes sense to implement.
- map() - Not implemented. Makes sense to implement.
- max() - Not implemented. Makes sense to implement.
- min() - Not implemented. Makes sense to implement.
- norm() - Not implemented. Makes sense to implement.
- pow() - Not implemented. Makes sense to implement.
- round() - Not implemented. Makes sense to implement.
- sq() - Not implemented. Makes sense to implement.
- sqrt() - Not implemented. Makes sense to implement.

#### Noise
- noise() - Not implemented. Makes sense to implement.
- noiseDetail() - Not implemented. Makes sense to implement.
- noiseSeed() - Not implemented. Makes sense to implement.

#### Random
- random() - Not implemented. Makes sense to implement.
- randomGaussian() - Not implemented. Makes sense to implement.
- randomSeed() - Not implemented. Makes sense to implement.

#### Trigonometry
- acos() - Not implemented. Makes sense to implement.
- angleMode() - Not implemented. Makes sense to implement.
- asin() - Not implemented. Makes sense to implement.
- atan() - Not implemented. Makes sense to implement.
- atan2() - Not implemented. Makes sense to implement.
- cos() - Not implemented. Makes sense to implement.
- degrees() - Not implemented. Makes sense to implement.
- radians() - Not implemented. Makes sense to implement.
- sin() - Not implemented. Makes sense to implement.
- tan() - Not implemented. Makes sense to implement.

#### Vector
- createVector() - Not implemented. Makes sense to implement.
- p5.Vector methods (add, angleBetween, array, clampToZero, copy, cross, dist, div, dot, equals, fromAngle, fromAngles, heading, lerp, limit, mag, magSq, mult, normalize, random2D, random3D, reflect, rem, rotate, set, setHeading, setMag, slerp, sub, toString, x, y, z) - Not implemented. Makes sense to implement.

### IO

- httpDo() - JS/Web-specific. No.
- httpGet() - JS/Web-specific. No.
- httpPost() - JS/Web-specific. No.
- loadBytes() - Not implemented. Makes sense to implement.
- loadJSON() - Not implemented. Makes sense to implement.
- loadStrings() - Not implemented. Makes sense to implement.
- loadTable() - Not implemented. Makes sense to implement.
- loadXML() - Not implemented. Makes sense to implement.

#### Output
- createWriter() - Not implemented. Makes sense to implement.
- p5.PrintWriter - Not implemented. Makes sense to implement.
- save() - Not implemented. Makes sense to implement.
- saveJSON() - Not implemented. Makes sense to implement.
- saveStrings() - Not implemented. Makes sense to implement.
- saveTable() - Not implemented. Makes sense to implement.

### Time & Date

- day() - Not implemented. Makes sense to implement.
- hour() - Not implemented. Makes sense to implement.
- millis() - Equivalent: SYSTEM_MILLIS
- minute() - Not implemented. Makes sense to implement.
- month() - Not implemented. Makes sense to implement.
- second() - Not implemented. Makes sense to implement.
- year() - Not implemented. Makes sense to implement.

#### p5.Table
- addColumn() - Not implemented. Makes sense to implement.
- addRow() - Not implemented. Makes sense to implement.
- clearRows() - Not implemented. Makes sense to implement.
- columns - Not implemented. Makes sense to implement.
- findRow() - Not implemented. Makes sense to implement.
- findRows() - Not implemented. Makes sense to implement.
- get() - Not implemented. Makes sense to implement.
- getArray() - Not implemented. Makes sense to implement.
- getColumn() - Not implemented. Makes sense to implement.
- getColumnCount() - Not implemented. Makes sense to implement.
- getNum() - Not implemented. Makes sense to implement.
- getObject() - Not implemented. Makes sense to implement.
- getRow() - Not implemented. Makes sense to implement.
- getRowCount() - Not implemented. Makes sense to implement.
- getRows() - Not implemented. Makes sense to implement.
- getString() - Not implemented. Makes sense to implement.
- matchRow() - Not implemented. Makes sense to implement.
- matchRows() - Not implemented. Makes sense to implement.
- removeColumn() - Not implemented. Makes sense to implement.
- removeRow() - Not implemented. Makes sense to implement.
- removeTokens() - Not implemented. Makes sense to implement.
- rows - Not implemented. Makes sense to implement.
- set() - Not implemented. Makes sense to implement.
- setNum() - Not implemented. Makes sense to implement.
- setString() - Not implemented. Makes sense to implement.
- trim() - Not implemented. Makes sense to implement.

#### p5.TableRow
- get() - Not implemented. Makes sense to implement.
- getNum() - Not implemented. Makes sense to implement.
- getString() - Not implemented. Makes sense to implement.
- set() - Not implemented. Makes sense to implement.
- setNum() - Not implemented. Makes sense to implement.
- setString() - Not implemented. Makes sense to implement.

#### p5.XML
- addChild() - Not implemented. Makes sense to implement.
- getAttributeCount() - Not implemented. Makes sense to implement.
- getChild() - Not implemented. Makes sense to implement.
- getChildren() - Not implemented. Makes sense to implement.
- getContent() - Not implemented. Makes sense to implement.
- getName() - Not implemented. Makes sense to implement.
- getNum() - Not implemented. Makes sense to implement.
- getParent() - Not implemented. Makes sense to implement.
- getString() - Not implemented. Makes sense to implement.
- hasAttribute() - Not implemented. Makes sense to implement.
- hasChildren() - Not implemented. Makes sense to implement.
- listAttributes() - Not implemented. Makes sense to implement.
- listChildren() - Not implemented. Makes sense to implement.
- removeChild() - Not implemented. Makes sense to implement.
- serialize() - Not implemented. Makes sense to implement.
- setAttribute() - Not implemented. Makes sense to implement.
- setContent() - Not implemented. Makes sense to implement.
- setName() - Not implemented. Makes sense to implement.

### Events

All event functions (Acceleration, Keyboard, Mouse, Touch) - JS/Web-specific. No.

### DOM

All DOM functions - JS/Web-specific. No.

### Data

#### Array Functions
- append() - Not implemented. Makes sense to implement.
- arrayCopy() - Not implemented. Makes sense to implement.
- concat() - Not implemented. Makes sense to implement.
- reverse() - Not implemented. Makes sense to implement.
- shorten() - Not implemented. Makes sense to implement.
- shuffle() - Not implemented. Makes sense to implement.
- sort() - Not implemented. Makes sense to implement.
- splice() - Not implemented. Makes sense to implement.
- subset() - Not implemented. Makes sense to implement.

#### Conversion
- boolean() - Not implemented. Makes sense to implement.
- byte() - Not implemented. Makes sense to implement.
- char() - Not implemented. Makes sense to implement.
- float() - Not implemented. Makes sense to implement.
- hex() - Not implemented. Makes sense to implement.
- int() - Not implemented. Makes sense to implement.
- str() - Not implemented. Makes sense to implement.
- unchar() - Not implemented. Makes sense to implement.
- unhex() - Not implemented. Makes sense to implement.

#### Dictionary
- createNumberDict() - Not implemented. Makes sense to implement.
- createStringDict() - Not implemented. Makes sense to implement.
- p5.StringDict - Not implemented. Makes sense to implement.

#### LocalStorage
- clearStorage() - Not implemented. Makes sense to implement.
- getItem() - Not implemented. Makes sense to implement.
- removeItem() - Not implemented. Makes sense to implement.
- storeItem() - Not implemented. Makes sense to implement.

#### String Functions
- join() - Not implemented. Makes sense to implement.
- match() - Not implemented. Makes sense to implement.
- matchAll() - Not implemented. Makes sense to implement.
- nf() - Not implemented. Makes sense to implement.
- nfc() - Not implemented. Makes sense to implement.
- nfp() - Not implemented. Makes sense to implement.
- nfs() - Not implemented. Makes sense to implement.
- split() - Not implemented. Makes sense to implement.
- splitTokens() - Not implemented. Makes sense to implement.
- trim() - Not implemented. Makes sense to implement.

#### p5.NumberDict
- add() - Not implemented. Makes sense to implement.
- div() - Not implemented. Makes sense to implement.
- maxKey() - Not implemented. Makes sense to implement.
- maxValue() - Not implemented. Makes sense to implement.
- minKey() - Not implemented. Makes sense to implement.
- minValue() - Not implemented. Makes sense to implement.
- mult() - Not implemented. Makes sense to implement.
- sub() - Not implemented. Makes sense to implement.

#### p5.TypedDict
- clear() - Not implemented. Makes sense to implement.
- create() - Not implemented. Makes sense to implement.
- get() - Not implemented. Makes sense to implement.
- hasKey() - Not implemented. Makes sense to implement.
- print() - Not implemented. Makes sense to implement.
- remove() - Not implemented. Makes sense to implement.
- saveJSON() - Not implemented. Makes sense to implement.
- saveTable() - Not implemented. Makes sense to implement.
- set() - Not implemented. Makes sense to implement.
- size() - Not implemented. Makes sense to implement.

### Structure

- disableFriendlyErrors - JS/Web-specific. No.
- draw() - Equivalent: guest export DRAW
- isLooping() - Not implemented. Makes sense to implement.
- loop() - Not implemented. Makes sense to implement.
- noLoop() - Not implemented. Makes sense to implement.
- pop() - Not implemented. Makes sense to implement.
- preload() - Not implemented. Makes sense to implement.
- push() - Not implemented. Makes sense to implement.
- redraw() - Not implemented. Makes sense to implement.
- remove() - Not implemented. Makes sense to implement.
- setup() - Equivalent: guest export SETUP

### Constants

All constants - Not implemented. Makes sense to implement.

### Foundation

All - JS/Web-specific. No.

## Raylib Methods Analysis

Raylib is a C library for game development. Many functions are similar to p5.js but focused on games.

### Core Module
- InitWindow, SetTargetFPS, WindowShouldClose, CloseWindow, etc. - Equivalent to GRAPHICS_SET_SIZE, etc. Makes sense to implement more window management.
- GetFrameTime, GetTime, etc. - Equivalent to SYSTEM_MILLIS. Makes sense to implement.
- SetConfigFlags, etc. - Not implemented. Makes sense to implement.

### Shapes Module
- DrawPixel, DrawLine, DrawCircle, DrawRectangle, etc. - Equivalent to GRAPHICS_POINT, GRAPHICS_LINE, etc.
- DrawTriangle, DrawPoly, etc. - Equivalent to GRAPHICS_TRIANGLE, etc.
- DrawSplineLinear, DrawSplineBasis, etc. - Equivalent to bezier functions.

### Textures Module
- LoadTexture, DrawTexture, etc. - Equivalent to GRAPHICS_PNG_REGISTER, GRAPHICS_PNG_DRAW_KEY.
- GenTextureColor, etc. - Not implemented. Makes sense to implement.

### Text Module
- DrawText, MeasureText, etc. - Equivalent to GRAPHICS_TEXT_KEY, GRAPHICS_TEXT_MEASURE_KEY.
- LoadFont, etc. - Equivalent to GRAPHICS_FONT_REGISTER_*.

### Models Module
- LoadModel, DrawModel, etc. - Equivalent to GRAPHICS_MESH_*.
- Camera functions - Equivalent to GRAPHICS_CAMERA_*.

### Audio Module
- InitAudioDevice, LoadSound, PlaySound, etc. - Equivalent to AUDIO_*.
- LoadMusicStream, PlayMusicStream, etc. - Not implemented. Makes sense to implement.

### Raymath Module
- Vector2Add, Vector2Subtract, etc. - Not implemented. Makes sense to implement (similar to p5.Vector).
- Matrix functions, Quaternion, etc. - Not implemented. Makes sense to implement.

What makes sense to implement: More advanced audio (music streams), vector math, matrix operations, more camera controls, lighting for 3D, shaders.

Equivalents: Many drawing and loading functions are already covered.

## MIDI Player/Synthesizer Implementation

To implement a MIDI player/synthesizer in wasm96:

- Add MIDI file parsing in the host.
- Implement a software synthesizer (e.g., using wavetables or simple oscillators).
- Add host imports for playing MIDI notes, setting instruments, etc.
- Integrate with the AUDIO_PUSH_SAMPLES for real-time synthesis.

Steps:
1. Add MIDI parsing library (e.g., Rust crate for MIDI).
2. Implement synthesizer with oscillators, envelopes, filters.
3. Add ABI functions like AUDIO_PLAY_MIDI, AUDIO_SET_INSTRUMENT, etc.
4. Update SDKs to expose these.

## Additional Features

## Tiled map support
- See <https://github.com/mapeditor/rs-tiled>

### Multi-ROM Support for Multi-Disk Games
- Allow loading multiple WASM modules or data blobs.
- Implement disk switching in the runtime.
- Add ABI for disk operations.

### BIOS Support for WASM-based 32-bit Operating Systems
- Extend WASI with more syscalls.
- Build on WASIp2 for advanced operations.
- Implement loading disk images and saving data to virtual drives.
- Create a default DOS-like OS with filesystem, c-compiler (compiled to WASM), text editor.
- Add hardware abstractions for storage and RAM.
