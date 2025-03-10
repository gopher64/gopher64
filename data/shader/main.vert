#version 450

layout(location = 0) out vec2 vUV;

void main()
{
	if (gl_VertexIndex == 0)
	{
		gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
	}
	else if (gl_VertexIndex == 1)
	{
		gl_Position = vec4(-1.0, +3.0, 0.0, 1.0);
	}
	else
	{
		gl_Position = vec4(+3.0, -1.0, 0.0, 1.0);
	}

	vUV = gl_Position.xy * 0.5 + 0.5;
}
