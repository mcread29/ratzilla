export function createProgram(
  gl: WebGL2RenderingContext,
  vertexSource: string,
  fragmentSource: string,
): WebGLProgram | null {
  const vertex = compileShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fragment = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource);
  if (!vertex || !fragment) return null;
  const program = gl.createProgram();
  if (!program) return null;
  gl.attachShader(program, vertex);
  gl.attachShader(program, fragment);
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.error(gl.getProgramInfoLog(program));
    return null;
  }
  return program;
}

export function compileShader(
  gl: WebGL2RenderingContext,
  type: number,
  source: string,
): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.error(gl.getShaderInfoLog(shader));
    return null;
  }
  return shader;
}

export function setUniform1f(gl: WebGL2RenderingContext, location: WebGLUniformLocation | null, value: number) {
  if (location != null) gl.uniform1f(location, value);
}

export function setUniform2f(gl: WebGL2RenderingContext, location: WebGLUniformLocation | null, x: number, y: number) {
  if (location != null) gl.uniform2f(location, x, y);
}

export function setUniform3f(
  gl: WebGL2RenderingContext,
  location: WebGLUniformLocation | null,
  value: [number, number, number],
) {
  if (location != null) gl.uniform3f(location, value[0], value[1], value[2]);
}
