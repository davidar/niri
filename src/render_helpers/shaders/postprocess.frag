uniform float noise;
uniform float saturation;
uniform vec4 bg_color;

// Mask the effect by the alpha of some other surface.
//
// niri_mask is 0.0 when masking is off, otherwise it is the reciprocal of the alpha at which the
// mask reaches full coverage.
uniform sampler2D niri_mask_tex;
uniform mat3 niri_input_to_mask;
uniform float niri_mask;

// Sin-less white noise by David Hoskins (MIT License).
// https://www.shadertoy.com/view/4djSRW
float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

vec3 saturate(vec3 color, float sat) {
    const vec3 w = vec3(0.2126, 0.7152, 0.0722);
    return mix(vec3(dot(color, w)), color, sat);
}

// Coverage of the masking surface at the current fragment.
//
// This is deliberately not the surface alpha itself: a half-translucent surface covers its pixels
// fully, it just lets some light through, so the effect below it must be drawn at full strength.
// Only fully transparent pixels get no effect, with a short ramp to keep client-side antialiased
// edges from turning into a staircase.
float niri_mask_coverage() {
    vec3 coords = niri_input_to_mask * vec3(v_coords, 1.0);
    if (coords.x < 0.0 || 1.0 < coords.x || coords.y < 0.0 || 1.0 < coords.y)
        return 0.0;

    return clamp(texture2D(niri_mask_tex, coords.xy).a * niri_mask, 0.0, 1.0);
}

vec4 postprocess(vec4 color) {
    if (saturation != 1.0) {
        color.rgb = saturate(color.rgb, saturation);
    }

    if (noise > 0.0) {
        vec2 uv = gl_FragCoord.xy;
        color.rgb += (hash12(uv) - 0.5) * noise;
    }

    // Mix bg_color behind the texture (both premultiplied alpha).
    color = color + bg_color * (1.0 - color.a);

    // Mask last: everything above is part of what gets masked away.
    if (niri_mask > 0.0) {
        color = color * niri_mask_coverage();
    }

    return color;
}
