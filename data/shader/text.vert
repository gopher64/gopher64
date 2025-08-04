#version 450 core

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable


layout (location = 0) in vec3 pos;      //  
layout (location = 1) in uint word;     // four chars
layout (location = 2) in vec3 word_pos; // where to place the word in screen space
layout (location = 3) in vec4 col_fg;   // foreground colour
layout (location = 4) in vec4 col_bg;   // background colour

struct per_word_data {
	uint msg;
	vec4 fg_colour;
	vec4 bg_colour;
};

// outputs 
layout (location = 0) out vec2 outTexCoord;
layout (location = 1) flat out per_word_data outMsg; // or maybe we could use a buffer?


// Override the built-in fixed function outputs
// to have more control over the SPIR-V code created.
out gl_PerVertex
{
    vec4 gl_Position;
};

// uniforms
layout (push_constant) uniform Params
{
	vec2 u_resolution;
};

void main() 
{

	// outMsg = 0x54455354; // this is where we can encode the message
	outMsg.msg = word;
	outMsg.fg_colour = col_fg;
	outMsg.bg_colour = col_bg;

	vec2 scale_factor = vec2(2.,4.)/(u_resolution);
	scale_factor *= vec2(1,0.5);

	outTexCoord = vec2((12 >> gl_VertexIndex) &1, (9 >> gl_VertexIndex ) &1);

	vec4 position = vec4(0,0,0,1);

	position.xy = vec2(-1, -1) + (pos.xy * word_pos.z + word_pos.xy) * scale_factor ;

	// vec4 position = vec4(pos * word_pos.z * vec3(scale_factor,0) + vec3(word_pos.xy,0),1);

	gl_Position = position;
}
