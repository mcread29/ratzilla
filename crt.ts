type CRTOptions = {
    curvature?: number
    scanlineStrength?: number
    maskStrength?: number
    vignetteStrength?: number
    aberration?: number
    bloom?: number
    copyFromDefault?: boolean
}

const quadVs = `#version 300 es
  precision highp float;
  
  out vec2 v_uv;
  
  const vec2 pos[3] = vec2[](
    vec2(-1.0, -1.0),
    vec2( 3.0, -1.0),
    vec2(-1.0,  3.0)
  );
  
  void main() {
    vec2 p = pos[gl_VertexID];
    v_uv = 0.5 * (p + 1.0);
    gl_Position = vec4(p, 0.0, 1.0);
  }
  `

const crtFs = `#version 300 es
  precision highp float;
  
  in vec2 v_uv;
  
  uniform sampler2D u_scene;
  uniform vec2 u_resolution;
  uniform float u_time;
  
  uniform float u_curvature;
  uniform float u_scanline_strength;
  uniform float u_mask_strength;
  uniform float u_vignette_strength;
  uniform float u_aberration;
  uniform float u_bloom;
  
  out vec4 out_color;
  
  vec2 curve_uv(vec2 uv, float k) {
    vec2 p = uv * 2.0 - 1.0;
    vec2 a = abs(p);
    p += p * (a.yx * a.yx) * k;
    return p * 0.5 + 0.5;
  }
  
  vec3 triad_mask(vec2 frag_coord) {
    float x = mod(frag_coord.x, 3.0);
    if (x < 1.0) return vec3(1.0, 0.65, 0.65);
    if (x < 2.0) return vec3(0.65, 1.0, 0.65);
    return vec3(0.65, 0.65, 1.0);
  }
  
  vec3 sample_scene(vec2 uv) {
    return texture(u_scene, uv).rgb;
  }
  
  void main() {
    vec2 frag = v_uv * u_resolution;
  
    vec2 uv = curve_uv(v_uv, u_curvature);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
      out_color = vec4(0.0, 0.0, 0.0, 1.0);
      return;
    }
  
    vec2 px = 1.0 / u_resolution;
  
    float ab = u_aberration;
    vec3 c;
    c.r = sample_scene(uv + vec2( ab, 0.0) * px).r;
    c.g = sample_scene(uv).g;
    c.b = sample_scene(uv + vec2(-ab, 0.0) * px).b;
  
    float scan = 0.5 + 0.5 * sin((frag.y + u_time * 60.0) * 3.14159265);
    float scan_mul = mix(1.0, 0.75 + 0.25 * scan, u_scanline_strength);
    c *= scan_mul;
  
    vec3 mask = triad_mask(frag);
    c *= mix(vec3(1.0), mask, u_mask_strength);
  
    float dx = px.x;
    float dy = px.y;
    vec3 bloom =
      sample_scene(uv + vec2( dx, 0.0)) +
      sample_scene(uv + vec2(-dx, 0.0)) +
      sample_scene(uv + vec2(0.0,  dy)) +
      sample_scene(uv + vec2(0.0, -dy));
    bloom *= 0.25;
    c = mix(c, max(c, bloom), u_bloom);
  
    vec2 d = v_uv * (1.0 - v_uv);
    float vig = d.x * d.y * 16.0;
    vig = pow(clamp(vig, 0.0, 1.0), 0.25);
    c *= mix(1.0, vig, u_vignette_strength);
  
    c = pow(c, vec3(1.0 / 2.2));
    out_color = vec4(c, 1.0);
  }
  `

function compileShader(
    gl: WebGL2RenderingContext,
    type: number,
    src: string
) {
    const sh = gl.createShader(type)!
    gl.shaderSource(sh, src)
    gl.compileShader(sh)
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
        const log = gl.getShaderInfoLog(sh) || "shader compile failed"
        gl.deleteShader(sh)
        throw new Error(log)
    }
    return sh
}

function createProgram(
    gl: WebGL2RenderingContext,
    vsSrc: string,
    fsSrc: string
) {
    const vs = compileShader(gl, gl.VERTEX_SHADER, vsSrc)
    const fs = compileShader(gl, gl.FRAGMENT_SHADER, fsSrc)
    const prog = gl.createProgram()!
    gl.attachShader(prog, vs)
    gl.attachShader(prog, fs)
    gl.linkProgram(prog)
    gl.deleteShader(vs)
    gl.deleteShader(fs)
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
        const log = gl.getProgramInfoLog(prog) || "program link failed"
        gl.deleteProgram(prog)
        throw new Error(log)
    }
    return prog
}

function createSceneTexture(
    gl: WebGL2RenderingContext,
    w: number,
    h: number
) {
    const tex = gl.createTexture()!
    gl.bindTexture(gl.TEXTURE_2D, tex)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
    gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA8,
        w,
        h,
        0,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        null
    )
    gl.bindTexture(gl.TEXTURE_2D, null)
    return tex
}

function createFbo(
    gl: WebGL2RenderingContext,
    tex: WebGLTexture
) {
    const fbo = gl.createFramebuffer()!
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbo)
    gl.framebufferTexture2D(
        gl.FRAMEBUFFER,
        gl.COLOR_ATTACHMENT0,
        gl.TEXTURE_2D,
        tex,
        0
    )
    const status = gl.checkFramebufferStatus(gl.FRAMEBUFFER)
    gl.bindFramebuffer(gl.FRAMEBUFFER, null)
    if (status !== gl.FRAMEBUFFER_COMPLETE) {
        gl.deleteFramebuffer(fbo)
        throw new Error(`fbo incomplete ${status}`)
    }
    return fbo
}

export class CRTPost {
    private gl: WebGL2RenderingContext
    private program: WebGLProgram
    private vao: WebGLVertexArrayObject
    private sceneTex: WebGLTexture | null = null
    private sceneFbo: WebGLFramebuffer | null = null
    private w = 1
    private h = 1
    private copyFromDefault: boolean

    private uScene: WebGLUniformLocation
    private uResolution: WebGLUniformLocation
    private uTime: WebGLUniformLocation
    private uCurvature: WebGLUniformLocation
    private uScan: WebGLUniformLocation
    private uMask: WebGLUniformLocation
    private uVig: WebGLUniformLocation
    private uAb: WebGLUniformLocation
    private uBloom: WebGLUniformLocation

    private opts: Required<
        Omit<CRTOptions, "copyFromDefault">
    >

    constructor(gl: WebGL2RenderingContext, opts: CRTOptions = {}) {
        this.gl = gl
        this.copyFromDefault = opts.copyFromDefault ?? false

        this.opts = {
            curvature: opts.curvature ?? 0.12,
            scanlineStrength: opts.scanlineStrength ?? 0.55,
            maskStrength: opts.maskStrength ?? 0.35,
            vignetteStrength: opts.vignetteStrength ?? 0.35,
            aberration: opts.aberration ?? 1.25,
            bloom: opts.bloom ?? 0.12,
        }

        this.program = createProgram(gl, quadVs, crtFs)
        this.vao = gl.createVertexArray()!
        gl.bindVertexArray(this.vao)
        gl.bindVertexArray(null)

        this.uScene = gl.getUniformLocation(this.program, "u_scene")!
        this.uResolution = gl.getUniformLocation(this.program, "u_resolution")!
        this.uTime = gl.getUniformLocation(this.program, "u_time")!
        this.uCurvature = gl.getUniformLocation(this.program, "u_curvature")!
        this.uScan = gl.getUniformLocation(this.program, "u_scanline_strength")!
        this.uMask = gl.getUniformLocation(this.program, "u_mask_strength")!
        this.uVig = gl.getUniformLocation(this.program, "u_vignette_strength")!
        this.uAb = gl.getUniformLocation(this.program, "u_aberration")!
        this.uBloom = gl.getUniformLocation(this.program, "u_bloom")!
    }

    resize(w: number, h: number) {
        const gl = this.gl
        if (w === this.w && h === this.h) return
        this.w = Math.max(1, w)
        this.h = Math.max(1, h)

        if (this.sceneFbo) gl.deleteFramebuffer(this.sceneFbo)
        if (this.sceneTex) gl.deleteTexture(this.sceneTex)

        this.sceneTex = createSceneTexture(gl, this.w, this.h)

        if (!this.copyFromDefault) {
            this.sceneFbo = createFbo(gl, this.sceneTex)
        } else {
            this.sceneFbo = null
        }
    }

    beginScene() {
        const gl = this.gl
        if (this.copyFromDefault) return
        if (!this.sceneFbo) throw new Error("missing scene fbo")
        gl.bindFramebuffer(gl.FRAMEBUFFER, this.sceneFbo)
        gl.viewport(0, 0, this.w, this.h)
    }

    endAndPresent(timeSeconds: number) {
        const gl = this.gl
        if (!this.sceneTex) throw new Error("missing scene texture")

        if (this.copyFromDefault) {
            gl.bindTexture(gl.TEXTURE_2D, this.sceneTex)
            gl.copyTexSubImage2D(
                gl.TEXTURE_2D,
                0,
                0,
                0,
                0,
                0,
                this.w,
                this.h
            )
            gl.bindTexture(gl.TEXTURE_2D, null)
        }

        gl.bindFramebuffer(gl.FRAMEBUFFER, null)
        gl.viewport(0, 0, this.w, this.h)

        gl.disable(gl.DEPTH_TEST)
        gl.disable(gl.BLEND)

        gl.useProgram(this.program)
        gl.bindVertexArray(this.vao)

        gl.activeTexture(gl.TEXTURE0)
        gl.bindTexture(gl.TEXTURE_2D, this.sceneTex)
        gl.uniform1i(this.uScene, 0)

        gl.uniform2f(this.uResolution, this.w, this.h)
        gl.uniform1f(this.uTime, timeSeconds)

        gl.uniform1f(this.uCurvature, this.opts.curvature)
        gl.uniform1f(this.uScan, this.opts.scanlineStrength)
        gl.uniform1f(this.uMask, this.opts.maskStrength)
        gl.uniform1f(this.uVig, this.opts.vignetteStrength)
        gl.uniform1f(this.uAb, this.opts.aberration)
        gl.uniform1f(this.uBloom, this.opts.bloom)

        gl.drawArrays(gl.TRIANGLES, 0, 3)

        gl.bindTexture(gl.TEXTURE_2D, null)
        gl.bindVertexArray(null)
        gl.useProgram(null)
    }
}