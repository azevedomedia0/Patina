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

// Draws the blurred scene inside a rounded rectangle, with the glass saturation.
// `uv_to_px` converts texture coordinates to screen pixels, `rect` is (x, y, w, h) in screen
// pixels, `radius` rounds all four corners and `saturation` is 1.0 for none.
uniform vec2 uv_to_px;
uniform vec4 rect;
uniform float radius;
uniform float saturation;

void main() {
    vec4 color = texture2D(tex, v_coords);
    float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    color.rgb = mix(vec3(luma), color.rgb, saturation);

    vec2 p = v_coords * uv_to_px - rect.xy;
    vec2 half_size = rect.zw * 0.5;
    vec2 q = abs(p - half_size) - (half_size - vec2(radius));
    float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
    float coverage = clamp(0.5 - d, 0.0, 1.0);

    // Premultiplied output; the blurred scene is opaque.
    color = vec4(color.rgb, 1.0) * coverage * alpha;

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
