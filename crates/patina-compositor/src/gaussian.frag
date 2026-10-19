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

// One direction of a separable Gaussian blur. `step` is the offset between taps in texture
// coordinates (one texel along the blur direction) and `sigma` the standard deviation in texels.
uniform vec2 step;
uniform float sigma;

void main() {
    vec4 sum = texture2D(tex, v_coords);
    float total = 1.0;
    for (int i = 1; i <= 24; i++) {
        float fi = float(i);
        if (fi > 3.0 * sigma) {
            break;
        }
        float w = exp(-0.5 * fi * fi / (sigma * sigma));
        sum += w * (texture2D(tex, v_coords + step * fi) + texture2D(tex, v_coords - step * fi));
        total += 2.0 * w;
    }
    vec4 color = sum / total;

#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0) * alpha;
#else
    color = color * alpha;
#endif

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
