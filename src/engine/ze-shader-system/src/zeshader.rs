use std::io::{BufReader, Read};
use std::str::{Chars, FromStr};
use ze_gfx::ShaderStageFlagBits;

pub struct Stage {
    pub stage: ShaderStageFlagBits,
    pub hlsl: String,
}

impl Stage {
    fn new(stage: ShaderStageFlagBits) -> Self {
        Self {
            stage,
            hlsl: String::new(),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum PassType {
    Graphics,
    Compute,
}

pub struct Pass {
    pub ty: PassType,
    pub name: String,
    pub common_hlsl: String,
    pub stages: Vec<Stage>,
}

impl Pass {
    fn new() -> Self {
        Self {
            ty: PassType::Graphics,
            name: String::new(),
            common_hlsl: String::new(),
            stages: vec![],
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum ParameterType {
    Uint,
    Uint64,
    Float,
    Float2,
    Float3,
    Float4,
    Float4x4,
    Texture2D,
    Sampler,
    ByteAddressBuffer,
    RWByteAddressBuffer,
}

impl FromStr for ParameterType {
    type Err = String;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "uint" => Ok(ParameterType::Uint),
            "uint64_t" => Ok(ParameterType::Uint64),
            "float" => Ok(ParameterType::Float),
            "float2" => Ok(ParameterType::Float2),
            "float3" => Ok(ParameterType::Float3),
            "float4" => Ok(ParameterType::Float4),
            "float4x4" => Ok(ParameterType::Float4x4),
            "Texture2D" => Ok(ParameterType::Texture2D),
            "Sampler" => Ok(ParameterType::Sampler),
            "ByteAddressBuffer" => Ok(ParameterType::ByteAddressBuffer),
            "RWByteAddressBuffer" => Ok(ParameterType::RWByteAddressBuffer),
            _ => Err(format!("Unknown type {}", str)),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Parameter {
    pub ty: ParameterType,
    pub name: String,
}

// Represents a prshd file, the text form of a shader
// prshd can also exist in a binary format (prshdbin), handled by the prshd module
pub struct Declaration {
    pub name: String,
    pub common_hlsl: String,
    pub passes: Vec<Pass>,
    pub parameters: Vec<Parameter>,
}

impl Declaration {
    pub fn from_read(read: Box<dyn Read>) -> Result<Self, String> {
        let mut buf_reader = BufReader::new(read);
        let mut content = String::new();
        buf_reader.read_to_string(&mut content).unwrap();

        Self::from_string(content)
    }

    pub fn from_string(content: String) -> Result<Self, String> {
        #[derive(PartialEq)]
        enum Block {
            Hlsl,
            Shader,
            Stage,
            Pass,
            Parameters,
        }

        let mut declaration = Declaration {
            name: "".to_string(),
            common_hlsl: "".to_string(),
            passes: vec![Pass::new()],
            parameters: vec![],
        };

        let mut blocks = vec![];
        let mut iter = content.chars();

        let mut current_pass_index = 0;
        let mut current_hlsl_stage = &mut declaration.common_hlsl;

        let mut is_in_stage = false;

        let skip_until = |iter: &mut Chars, char: char| -> bool {
            let mut current_char = match iter.next() {
                None => return false,
                Some(char) => char,
            };

            while current_char != char {
                current_char = match iter.next() {
                    None => return false,
                    Some(char) => char,
                };
            }

            true
        };

        loop {
            let mut char = match iter.next() {
                None => break,
                Some(ch) => ch,
            };

            if char.is_alphabetic() {
                let mut word = String::new();
                loop {
                    word.push(char);
                    char = iter.next().unwrap();
                    if !char.is_alphanumeric() && char != '_' {
                        break;
                    }
                }

                // Parameter parsing
                // Treat "word" as the name of the parameter
                if !blocks.is_empty() && *blocks.last().unwrap() == Block::Parameters {
                    if !skip_until(&mut iter, ':') {
                        return Err("Can't find type for parameter.".to_string());
                    }

                    let mut ty = String::new();
                    loop {
                        if char != ' ' {
                            ty.push(char);
                        }

                        char = match iter.next() {
                            None => return Err("Encountered EOF.".to_string()),
                            Some(ch) => ch,
                        };

                        if !char.is_alphanumeric() && char != '_' && char != ' ' {
                            break;
                        }
                    }

                    if char != ';' && !skip_until(&mut iter, ';') {
                        return Err("Parameter must finished with a semi-colon.".to_string());
                    }

                    declaration.parameters.push(Parameter::new(
                        ParameterType::from_str(&ty).unwrap(),
                        word.clone(),
                    ));
                } else if word == "shader" {
                    if !skip_until(&mut iter, '"') {
                        return Err(
                            "Can't properly parse shader name. Excepted syntax: 'shader \"Name\"'"
                                .to_string(),
                        );
                    }

                    loop {
                        char = iter.next().unwrap();
                        match char
                        {
                            '"' => break,
                            '\n' => return Err("Can't properly parse shader name. Excepted syntax: 'shader \"Name\"".to_string()),
                            _ => declaration.name.push(char)
                        }
                    }

                    if !skip_until(&mut iter, '{') {
                        return Err("Shader block neved opened.".to_string());
                    }

                    blocks.push(Block::Shader);
                } else if word == "vertex" && !is_in_stage {
                    if !skip_until(&mut iter, '{') {
                        return Err("Vertex block never opened.".to_string());
                    }

                    if declaration.passes[current_pass_index].ty == PassType::Compute {
                        return Err("Cannot add a vertex block to a compute pass.".to_string());
                    }

                    blocks.push(Block::Stage);
                    declaration.passes[current_pass_index]
                        .stages
                        .push(Stage::new(ShaderStageFlagBits::Vertex));
                    current_hlsl_stage = &mut declaration.passes[current_pass_index]
                        .stages
                        .last_mut()
                        .unwrap()
                        .hlsl;
                    is_in_stage = true;
                } else if word == "fragment" && !is_in_stage {
                    if !skip_until(&mut iter, '{') {
                        return Err("Fragment block never opened.".to_string());
                    }

                    if declaration.passes[current_pass_index].ty == PassType::Compute {
                        return Err("Cannot add a fragment block to a compute pass.".to_string());
                    }

                    blocks.push(Block::Stage);
                    declaration.passes[current_pass_index]
                        .stages
                        .push(Stage::new(ShaderStageFlagBits::Fragment));
                    current_hlsl_stage = &mut declaration.passes[current_pass_index]
                        .stages
                        .last_mut()
                        .unwrap()
                        .hlsl;
                    is_in_stage = true;
                } else if word == "compute" && !is_in_stage {
                    if !skip_until(&mut iter, '{') {
                        return Err("Compute block never opened.".to_string());
                    }

                    if !declaration.passes[current_pass_index].stages.is_empty() {
                        return Err("Compute block already detected or pass is a graphical one."
                            .to_string());
                    }

                    blocks.push(Block::Stage);
                    declaration.passes[current_pass_index]
                        .stages
                        .push(Stage::new(ShaderStageFlagBits::Compute));
                    declaration.passes[current_pass_index].ty = PassType::Compute;
                    current_hlsl_stage = &mut declaration.passes[current_pass_index]
                        .stages
                        .last_mut()
                        .unwrap()
                        .hlsl;
                    is_in_stage = true;
                } else if word == "pass" {
                    if !skip_until(&mut iter, '"') {
                        return Err("Invalid pass syntax. 'pass \"Name\"'".to_string());
                    }

                    declaration.passes.push(Pass::new());
                    current_pass_index = declaration.passes.len() - 1;

                    loop {
                        let char = match iter.next() {
                            None => break,
                            Some(ch) => ch,
                        };

                        match char {
                            '"' => break,
                            '\n' => {
                                return Err(
                                    "Can't properly parse pass name. 'pass \"Name\"'".to_string()
                                )
                            }
                            _ => declaration.passes[current_pass_index].name.push(char),
                        }
                    }

                    current_hlsl_stage = &mut declaration.passes[current_pass_index].common_hlsl;

                    if !skip_until(&mut iter, '{') {
                        return Err("Pass block never opened.".to_string());
                    }

                    blocks.push(Block::Pass);
                } else if word == "parameters" {
                    if !skip_until(&mut iter, '{') {
                        return Err("Parameters block never opened.".to_string());
                    }

                    blocks.push(Block::Parameters);
                } else {
                    current_hlsl_stage.push_str(&word);
                    current_hlsl_stage.push(char);
                }
            } else if char == '{' {
                blocks.push(Block::Hlsl);
                current_hlsl_stage.push(char);
            } else if char == '}' {
                let block = blocks.pop().unwrap();

                if block == Block::Pass {
                    current_hlsl_stage = &mut declaration.common_hlsl;
                    current_pass_index = 0;
                } else if block == Block::Stage {
                    current_hlsl_stage = &mut declaration.passes.last_mut().unwrap().common_hlsl;
                    is_in_stage = false;
                } else if block != Block::Parameters && block != Block::Shader {
                    current_hlsl_stage.push(char);
                }
            } else {
                current_hlsl_stage.push(char);
            }
        }

        assert!(blocks.is_empty());
        Ok(declaration)
    }
}

impl Parameter {
    pub fn new(ty: ParameterType, name: String) -> Self {
        Self { ty, name }
    }

    pub fn is_uav(&self) -> bool {
        matches!(self.ty, ParameterType::RWByteAddressBuffer)
    }
}

#[cfg(test)]
mod tests {
    use crate::zeshader::{Declaration, Parameter, ParameterType, PassType};

    #[test]
    fn parse_single_pass_one_compute() {
        let file = "
        shader \"SimpleCompute\"
        {
            compute
            {
                
            }
        }
        "
        .to_string();

        let declaration = Declaration::from_string(file).unwrap();
        assert_eq!(declaration.name, "SimpleCompute");
        assert!(declaration.parameters.is_empty());
    }

    #[test]
    fn parse_single_pass_one_compute_parameters() {
        let file = "
        shader \"SimpleCompute\"
        {
            parameters
            {
                a : uint;
                b : uint64_t;
                c : float;
                d : float2;
                e : float3;
                f : float4;
                g : float4x4;
                h : Texture2D;
                i : Sampler;
                j : ByteAddressBuffer;
                k : RWByteAddressBuffer;
            }

            compute
            {
                
            }
        }
        "
        .to_string();

        let declaration = Declaration::from_string(file).unwrap();
        assert_eq!(declaration.name, "SimpleCompute");
        assert_eq!(
            declaration.parameters[0],
            Parameter::new(ParameterType::Uint, "a".to_string())
        );
        assert_eq!(
            declaration.parameters[1],
            Parameter::new(ParameterType::Uint64, "b".to_string())
        );
        assert_eq!(
            declaration.parameters[2],
            Parameter::new(ParameterType::Float, "c".to_string())
        );
        assert_eq!(
            declaration.parameters[3],
            Parameter::new(ParameterType::Float2, "d".to_string())
        );
        assert_eq!(
            declaration.parameters[4],
            Parameter::new(ParameterType::Float3, "e".to_string())
        );
        assert_eq!(
            declaration.parameters[5],
            Parameter::new(ParameterType::Float4, "f".to_string())
        );
        assert_eq!(
            declaration.parameters[6],
            Parameter::new(ParameterType::Float4x4, "g".to_string())
        );
        assert_eq!(
            declaration.parameters[7],
            Parameter::new(ParameterType::Texture2D, "h".to_string())
        );
        assert_eq!(
            declaration.parameters[8],
            Parameter::new(ParameterType::Sampler, "i".to_string())
        );
        assert_eq!(
            declaration.parameters[9],
            Parameter::new(ParameterType::ByteAddressBuffer, "j".to_string())
        );
        assert_eq!(
            declaration.parameters[10],
            Parameter::new(ParameterType::RWByteAddressBuffer, "k".to_string())
        );
    }

    #[test]
    fn parse_two_pass_one_compute_one_graphics() {
        let file = "
        shader \"SimpleCompute\"
        {
            compute
            {
                
            }

            pass \"pass0\"
            {
                vertex
                {
                }
            }
        }
        "
        .to_string();

        let declaration = Declaration::from_string(file).unwrap();
        assert_eq!(declaration.name, "SimpleCompute");
        assert!(declaration.parameters.is_empty());
        assert_eq!(declaration.passes.len(), 2);
        assert_eq!(declaration.passes[0].ty, PassType::Compute);
        assert_eq!(declaration.passes[1].name, "pass0");
        assert_eq!(declaration.passes[1].ty, PassType::Graphics);
    }

    #[test]
    #[should_panic(expected = "Compute block already detected or pass is a graphical one.")]
    fn parse_two_pass_one_compute_one_graphics_with_compute_panic() {
        let file = "
        shader \"SimpleCompute\"
        {
            compute
            {
                
            }

            pass \"pass0\"
            {
                vertex
                {
                }
                compute
                {
                }
            }
        }
        "
        .to_string();

        Declaration::from_string(file).unwrap();
    }

    #[test]
    #[should_panic(expected = "Cannot add a vertex block to a compute pass.")]
    fn parse_single_pass_one_compute_one_vertex_panic() {
        let file = "
        shader \"SimpleComputeOneVertex\"
        {
            compute
            {
                
            }
    
            vertex
            {
            
            }
        }
        "
        .to_string();

        Declaration::from_string(file).unwrap();
    }

    #[test]
    #[should_panic(expected = "Cannot add a fragment block to a compute pass.")]
    fn parse_single_pass_one_compute_one_fragment_panic() {
        let file = "
        shader \"SimpleComputeOneFragment\"
        {
            compute
            {
                
            }
    
            fragment
            {
            
            }
        }
        "
        .to_string();

        Declaration::from_string(file).unwrap();
    }
}
