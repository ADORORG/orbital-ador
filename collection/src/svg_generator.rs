use serde_json::{Value, json};
use anyhow::{anyhow, Result};

const ENCODED_TRAITS_JSON: &str = include_str!("encoded_traits.json");
const SVG_TEMPLATES_JSON: &str = include_str!("svg-templates.json");

pub struct SvgGenerator;

impl SvgGenerator {
  fn get_encoded_traits() -> Value {
    serde_json::from_str(ENCODED_TRAITS_JSON).unwrap()
  }

  fn get_svg_templates() -> Value {
    serde_json::from_str(SVG_TEMPLATES_JSON).unwrap()
  }

  pub fn decode_traits(index: u128) -> Result<(String, String, String, String, String, String, String, String)> {
    let encoded_traits = Self::get_encoded_traits();
    let traits_array = encoded_traits["traits"].as_array()
      .ok_or_else(|| anyhow!("Invalid traits array"))?;
    let encoded_trait = traits_array.get(index as usize)
      .ok_or_else(|| anyhow!("Invalid trait index"))?
      .as_str()
      .ok_or_else(|| anyhow!("Invalid trait format"))?;

    let encoded = encoded_trait.parse::<u128>()
      .map_err(|e| anyhow!("Failed to parse encoded trait: {}", e))?;

    let format = &encoded_traits["format"];
    let bg_bits = format["bgBits"].as_u64().unwrap() as u32;
    let outer_eyes_bits = format["outerEyesBits"].as_u64().unwrap() as u32;
    let nose_bits = format["noseBits"].as_u64().unwrap() as u32;
    let mouth_bits = format["mouthBits"].as_u64().unwrap() as u32;
    let eyes_bits = format["eyesBits"].as_u64().unwrap() as u32;
    let head_acc_bits = format["headAccBits"].as_u64().unwrap() as u32;
    let body_acc_bits = format["bodyAccBits"].as_u64().unwrap() as u32;
    let species_bits = format["speciesBits"].as_u64().unwrap() as u32;

    let bg_code = (encoded & ((1u128 << bg_bits) - 1)) as usize;
    let outer_eyes_code = ((encoded >> bg_bits) & ((1u128 << outer_eyes_bits) - 1)) as usize;
    let nose_code = ((encoded >> (bg_bits + outer_eyes_bits)) & ((1u128 << nose_bits) - 1)) as usize;
    let mouth_code = ((encoded >> (bg_bits + outer_eyes_bits + nose_bits)) & ((1u128 << mouth_bits) - 1)) as usize;
    let eyes_code = ((encoded >> (bg_bits + outer_eyes_bits + nose_bits + mouth_bits)) & ((1u128 << eyes_bits) - 1)) as usize;
    let head_acc_code = ((encoded >> (bg_bits + outer_eyes_bits + nose_bits + mouth_bits + eyes_bits)) & ((1u128 << head_acc_bits) - 1)) as usize;
    let body_acc_code = ((encoded >> (bg_bits + outer_eyes_bits + nose_bits + mouth_bits + eyes_bits + head_acc_bits)) & ((1u128 << body_acc_bits) - 1)) as usize;
    let species_code = ((encoded >> (bg_bits + outer_eyes_bits + nose_bits + mouth_bits + eyes_bits + head_acc_bits + body_acc_bits)) & ((1u128 << species_bits) - 1)) as usize;

    let indices = &encoded_traits["indices"];
    let species = indices["species"][species_code].as_str().unwrap().to_string();
    let body_acc = indices["bodyAccessories"][body_acc_code].as_str().unwrap().to_string();
    let head_acc = indices["headAccessories"][head_acc_code].as_str().unwrap().to_string();
    let eyes = indices["eyes"][eyes_code].as_str().unwrap().to_string();
    let mouth = indices["mouth"][mouth_code].as_str().unwrap().to_string();
    let nose = indices["nose"][nose_code].as_str().unwrap().to_string();
    let outer_eyes = indices["outerEyes"][outer_eyes_code].as_str().unwrap().to_string();
    let background = indices["background"][bg_code].as_str().unwrap().to_string();

    Ok((species, background, body_acc, head_acc, eyes, mouth, nose, outer_eyes))
  }

  pub fn get_attributes(index: u128) -> Result<String> {
    let (species, background, body_acc, head_acc, eyes, mouth, _nose, _outer_eyes) = Self::decode_traits(index)?;

    let attributes = json!({
      "species": species,
      "background": background,
      "body": body_acc,
      "head": head_acc,
      "eyes": eyes,
      "mouth": mouth,
    });

    Ok(attributes.to_string())
  }

  pub fn generate_svg(index: u128) -> Result<String> {
    let (species, background, body_acc, head_acc, eyes, mouth, nose, outer_eyes) = Self::decode_traits(index)?;

    let svg_templates = Self::get_svg_templates();

    let mut svg = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg width=\"100%\" height=\"100%\" viewBox=\"0 0 200 200\" xmlns=\"http://www.w3.org/2000/svg\">\n");
    
    svg.push_str(svg_templates["background"][&background].as_str().unwrap());
    svg.push_str("\n");
    
    let species_template = &svg_templates["species"][&species];
    svg.push_str(species_template["body"].as_str().unwrap());
    svg.push_str("\n");
    svg.push_str(svg_templates["nipples"]["normal"].as_str().unwrap());
    svg.push_str("\n");
    
    if body_acc != "none" {
      svg.push_str(svg_templates["bodyAccessories"][&body_acc].as_str().unwrap());
      svg.push_str("\n");
    }
    
    svg.push_str(species_template["ears"].as_str().unwrap());
    svg.push_str("\n");
    svg.push_str(species_template["head"].as_str().unwrap());
    svg.push_str("\n");
    
    if head_acc != "none" {
      svg.push_str(svg_templates["headAccessories"][&head_acc].as_str().unwrap());
      svg.push_str("\n");
    }
    
    svg.push_str(svg_templates["nose"][&nose].as_str().unwrap());
    svg.push_str("\n");
    svg.push_str(svg_templates["outerEyes"][&outer_eyes].as_str().unwrap());
    svg.push_str("\n");
    svg.push_str(svg_templates["eyes"][&eyes].as_str().unwrap());
    svg.push_str("\n");
    svg.push_str(svg_templates["mouth"][&mouth].as_str().unwrap());
    svg.push_str("\n");
    
    svg.push_str("</svg>");

    Ok(svg)
  }
} 