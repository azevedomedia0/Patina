#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

uniform float alpha;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

// Window content size, this element's offset inside it, and this element's size, all in
// physical pixels. `radius` rounds the bottom two corners of the window content; the top
// corners belong to the titlebar above.
uniform vec2 win_size;
uniform vec2 elem_origin;
uniform vec2 elem_size;
uniform float radius;

void main() {
    vec4 color = texture2D(tex, v_coords);

#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0) * alpha;
#else
    color = color * alpha;
#endif

    vec2 p = elem_origin + v_coords * elem_size;
    float coverage = 1.0;
    if (p.y > win_size.y - radius) {
        if (p.x < radius) {
            coverage = clamp(0.5 - (length(p - vec2(radius, win_size.y - radius)) - radius), 0.0, 1.0);
        } else if (p.x > win_size.x - radius) {
            coverage = clamp(0.5 - (length(p - vec2(win_size.x - radius, win_size.y - radius)) - radius), 0.0, 1.0);
        }
    }
    color *= coverage;

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
