use clap::Parser;
use e2k::*;
use std::process;

fn main() {
    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse CLI arguments
    let args = Cli::parse();

    // Set debug logging if requested
    if args.debug {
        log::set_max_level(log::LevelFilter::Debug);
    }

    // Run the conversion
    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(args: Cli) -> error::Result<()> {
    // Validate arguments
    args.validate()?;

    log::info!("Starting conversion for LCSC ID: {}", args.lcsc_id);

    // Setup output directories
    let lib_manager = LibraryManager::new(&args.output);
    lib_manager.create_directories()?;

    // Fetch component data from EasyEDA API
    let api = EasyedaApi::new();
    let component_data = api.get_component_data(&args.lcsc_id)?;

    log::info!("Fetched component: {}", component_data.title);

    // Process symbol (if requested)
    if args.symbol || args.full {
        log::info!("Converting symbol...");

        let ee_symbol = SymbolImporter::parse(&component_data.data_str)?;

        // Convert EasyEDA symbol to KiCad symbol
        let footprint_name = sanitize_name(&component_data.title);
        let mut ki_symbol = kicad::KiSymbol {
            name: sanitize_name(&component_data.title),
            reference: ee_symbol.prefix.clone(),
            value: component_data.title.clone(),
            footprint: format!("e2k:{}", footprint_name),
            datasheet: component_data.datasheet.clone(),
            manufacturer: component_data.manufacturer.clone(),
            lcsc_id: component_data.lcsc_id.clone(),
            jlc_id: component_data.jlc_id.clone(),
            pins: Vec::new(),
            rectangles: Vec::new(),
            circles: Vec::new(),
            arcs: Vec::new(),
            polylines: Vec::new(),
        };

        // Convert pins with bbox adjustment
        let _converter = Converter::new(args.kicad_version());

        log::debug!("bbox_x = {}, bbox_y = {}", component_data.bbox_x, component_data.bbox_y);

        for ee_pin in &ee_symbol.pins {
            let adjusted_x = ee_pin.x - component_data.bbox_x;
            let adjusted_y = ee_pin.y - component_data.bbox_y;

            if ee_pin.name.contains("PG10") {
                log::info!("PG10 pin: raw x={}, y={}, adjusted x={}, y={}, final y={}",
                    ee_pin.x, ee_pin.y, adjusted_x, adjusted_y, -adjusted_y);
            }

            // Log pins with unusual length
            if ee_pin.length >= 100.0 {
                log::warn!("Pin {} ({}) has unusual length: {}", ee_pin.number, ee_pin.name, ee_pin.length);
            }

            ki_symbol.pins.push(kicad::KiPin {
                number: ee_pin.number.clone(),
                name: ee_pin.name.clone(),
                pin_type: kicad::PinType::from_easyeda(&ee_pin.electric_type),
                style: if ee_pin.dot {
                    kicad::PinStyle::Inverted
                } else if ee_pin.clock {
                    kicad::PinStyle::Clock
                } else {
                    kicad::PinStyle::Line
                },
                pos_x: adjusted_x,
                pos_y: -adjusted_y,  // Back to negation to test
                rotation: ee_pin.rotation,
                length: ee_pin.length,
            });
        }

        // Convert rectangles with bbox adjustment
        for (idx, ee_rect) in ee_symbol.rectangles.iter().enumerate() {
            let adjusted_x = ee_rect.x - component_data.bbox_x;
            let adjusted_y = component_data.bbox_y - ee_rect.y;  // bbox_y - pos_y
            let adjusted_x2 = (ee_rect.x + ee_rect.width) - component_data.bbox_x;
            let adjusted_y2 = component_data.bbox_y - (ee_rect.y + ee_rect.height);  // bbox_y - (pos_y + height)

            // First rectangle is usually the main body, should be filled
            let fill = if idx == 0 { true } else { ee_rect.fill };

            ki_symbol.rectangles.push(kicad::KiRectangle {
                x1: adjusted_x,
                y1: adjusted_y,  // No negation
                x2: adjusted_x2,
                y2: adjusted_y2,  // No negation
                stroke_width: ee_rect.stroke_width,
                fill,
            });
        }

        // Convert circles with bbox adjustment
        for ee_circle in &ee_symbol.circles {
            let adjusted_cx = ee_circle.cx - component_data.bbox_x;
            let adjusted_cy = component_data.bbox_y - ee_circle.cy;  // bbox_y - pos_y

            ki_symbol.circles.push(kicad::KiCircle {
                cx: adjusted_cx,
                cy: adjusted_cy,  // No negation
                radius: ee_circle.radius,
                stroke_width: ee_circle.stroke_width,
                fill: ee_circle.fill,
            });
        }

        // Convert ellipses with bbox adjustment
        // If rx == ry, treat as circle; otherwise, approximate as circle with average radius
        for ee_ellipse in &ee_symbol.ellipses {
            let adjusted_cx = ee_ellipse.cx - component_data.bbox_x;
            let adjusted_cy = component_data.bbox_y - ee_ellipse.cy;  // bbox_y - pos_y

            // Use average of rx and ry as radius (or just rx if they're equal)
            let radius = (ee_ellipse.rx + ee_ellipse.ry) / 2.0;

            ki_symbol.circles.push(kicad::KiCircle {
                cx: adjusted_cx,
                cy: adjusted_cy,  // No negation
                radius,
                stroke_width: ee_ellipse.stroke_width,
                fill: ee_ellipse.fill,
            });
        }

        // Convert polylines with bbox adjustment
        for ee_polyline in &ee_symbol.polylines {
            let adjusted_points: Vec<(f64, f64)> = ee_polyline.points.iter()
                .map(|(x, y)| {
                    let adj_x = x - component_data.bbox_x;
                    let adj_y = component_data.bbox_y - y;  // bbox_y - pos_y
                    (adj_x, adj_y)  // No negation
                })
                .collect();

            ki_symbol.polylines.push(kicad::KiPolyline {
                points: adjusted_points,
                stroke_width: ee_polyline.stroke_width,
                fill: false,
            });
        }

        // Convert paths to polylines with bbox adjustment
        // Parse SVG path commands (M, L) and convert to polylines
        for ee_path in &ee_symbol.paths {
            let path_str = &ee_path.path_data;
            let tokens: Vec<&str> = path_str.split_whitespace().collect();
            let mut points = Vec::new();
            let mut i = 0;

            while i < tokens.len() {
                let token = tokens[i];
                match token {
                    "M" | "L" => {
                        // Move or Line command, followed by x,y coordinates
                        if i + 1 < tokens.len() {
                            i += 1;
                            // Parse coordinate pair (may be "x,y" or separate "x" "y")
                            let coord_str = tokens[i];
                            if let Some((x_str, y_str)) = coord_str.split_once(',') {
                                if let (Ok(x), Ok(y)) = (x_str.parse::<f64>(), y_str.parse::<f64>()) {
                                    let adj_x = x - component_data.bbox_x;
                                    let adj_y = component_data.bbox_y - y;
                                    points.push((adj_x, adj_y));
                                }
                            } else if i + 1 < tokens.len() {
                                // Separate x and y
                                if let (Ok(x), Ok(y)) = (tokens[i].parse::<f64>(), tokens[i + 1].parse::<f64>()) {
                                    let adj_x = x - component_data.bbox_x;
                                    let adj_y = component_data.bbox_y - y;
                                    points.push((adj_x, adj_y));
                                    i += 1;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            if points.len() >= 2 {
                ki_symbol.polylines.push(kicad::KiPolyline {
                    points,
                    stroke_width: ee_path.stroke_width,
                    fill: ee_path.fill,
                });
            }
        }

        // Export symbol
        let exporter = SymbolExporter::new(args.kicad_version());
        let symbol_data = exporter.export(&ki_symbol)?;

        let lib_path = lib_manager.get_symbol_lib_path(args.v5);

        if args.overwrite && lib_manager.component_exists(&lib_path, &ki_symbol.name)? {
            log::info!("Updating existing symbol: {}", ki_symbol.name);
            lib_manager.update_component(&lib_path, &ki_symbol.name, &symbol_data)?;
        } else {
            log::info!("Adding new symbol: {}", ki_symbol.name);
            lib_manager.add_component(&lib_path, &symbol_data)?;
        }

        println!("✓ Symbol converted: {}", ki_symbol.name);
    }

    // Process footprint (if requested)
    if args.footprint || args.full {
        log::info!("Converting footprint...");

        let ee_footprint = FootprintImporter::parse(&component_data.package_detail)?;
        let _converter = Converter::new(args.kicad_version());

        // Convert EasyEDA footprint to KiCad footprint
        let mut ki_footprint = kicad::KiFootprint {
            name: sanitize_name(&component_data.title),
            pads: Vec::new(),
            tracks: Vec::new(),
            circles: Vec::new(),
            arcs: Vec::new(),
            texts: Vec::new(),
            lines: Vec::new(),
            model_3d: None,
        };

        // Convert pads with bbox adjustment
        for ee_pad in &ee_footprint.pads {
            let pad_type = if ee_pad.hole_radius.is_some() {
                kicad::PadType::ThroughHole
            } else {
                kicad::PadType::Smd
            };

            // Use layer mapping based on pad type
            let layers = if pad_type == kicad::PadType::ThroughHole {
                kicad::map_pad_layers_tht(ee_pad.layer_id)
            } else {
                kicad::map_pad_layers_smd(ee_pad.layer_id)
            };

            // Create drill for through-hole pads
            let drill = if let Some(hole_radius) = ee_pad.hole_radius {
                if let Some(hole_length) = ee_pad.hole_length {
                    // Elliptical drill
                    let max_distance_hole = (hole_radius * 2.0).max(hole_length);
                    let pos_0 = ee_pad.height - max_distance_hole;
                    let pos_90 = ee_pad.width - max_distance_hole;

                    if pos_0 > pos_90 {
                        // Vertical orientation
                        Some(kicad::Drill {
                            diameter: hole_radius * 2.0,
                            width: Some(hole_length),
                            offset_x: 0.0,
                            offset_y: 0.0,
                        })
                    } else {
                        // Horizontal orientation
                        Some(kicad::Drill {
                            diameter: hole_length,
                            width: Some(hole_radius * 2.0),
                            offset_x: 0.0,
                            offset_y: 0.0,
                        })
                    }
                } else {
                    // Circular drill
                    Some(kicad::Drill {
                        diameter: hole_radius * 2.0,
                        width: None,
                        offset_x: 0.0,
                        offset_y: 0.0,
                    })
                }
            } else {
                None
            };

            // Apply bbox normalization for footprint coordinates
            let adjusted_x = ee_pad.x - component_data.package_bbox_x;
            let adjusted_y = ee_pad.y - component_data.package_bbox_y;

            // Handle polygon pads
            let (size_x, size_y, rotation, polygon) = if ee_pad.shape == "POLYGON" && !ee_pad.points.is_empty() {
                // Parse points: space-separated x y coordinates
                let coords: Vec<f64> = ee_pad.points
                    .split_whitespace()
                    .filter_map(|s| s.parse::<f64>().ok())
                    .collect();

                if coords.len() >= 4 {  // At least 2 points (x,y pairs)
                    // Generate polygon with coordinates relative to pad position
                    let mut poly_str = String::from("\n\t\t(primitives \n\t\t\t(gr_poly \n\t\t\t\t(pts");

                    for i in (0..coords.len()).step_by(2) {
                        if i + 1 < coords.len() {
                            let rel_x = coords[i] - component_data.package_bbox_x - adjusted_x;
                            let rel_y = coords[i + 1] - component_data.package_bbox_y - adjusted_y;
                            poly_str.push_str(&format!(" (xy {:.2} {:.2})", rel_x, rel_y));
                        }
                    }

                    poly_str.push_str("\n\t\t\t\t) \n\t\t\t\t(width 0.1) \n\t\t\t)\n\t\t)\n\t");

                    // Set minimal pad size and force orientation to 0
                    (0.005, 0.005, 0.0, Some(poly_str))
                } else {
                    (ee_pad.width, ee_pad.height, ee_pad.rotation, None)
                }
            } else {
                (ee_pad.width, ee_pad.height, ee_pad.rotation, None)
            };

            ki_footprint.pads.push(kicad::KiPad {
                number: ee_pad.number.clone(),
                pad_type,
                shape: kicad::PadShape::from_easyeda(&ee_pad.shape),
                pos_x: adjusted_x,
                pos_y: adjusted_y,
                size_x,
                size_y,
                rotation,
                layers,
                drill,
                polygon,
            });
        }

        // Convert tracks to lines with bbox adjustment
        for ee_track in &ee_footprint.tracks {
            let adjusted_x1 = ee_track.x1 - component_data.package_bbox_x;
            let adjusted_y1 = ee_track.y1 - component_data.package_bbox_y;
            let adjusted_x2 = ee_track.x2 - component_data.package_bbox_x;
            let adjusted_y2 = ee_track.y2 - component_data.package_bbox_y;

            ki_footprint.lines.push(kicad::KiLine {
                start_x: adjusted_x1,
                start_y: adjusted_y1,
                end_x: adjusted_x2,
                end_y: adjusted_y2,
                width: ee_track.width,
                layer: kicad::map_layer(ee_track.layer_id),
            });
        }

        // Convert circles with bbox adjustment
        for ee_circle in &ee_footprint.circles {
            let adjusted_cx = ee_circle.cx - component_data.package_bbox_x;
            let adjusted_cy = ee_circle.cy - component_data.package_bbox_y;

            ki_footprint.circles.push(kicad::FootprintKiCircle {
                center_x: adjusted_cx,
                center_y: adjusted_cy,
                end_x: adjusted_cx + ee_circle.radius,
                end_y: adjusted_cy,
                width: ee_circle.stroke_width,
                layer: "F.SilkS".to_string(),
                fill: ee_circle.fill,
            });
        }

        // Add 3D model reference if available
        if let Some(model_info) = &component_data.model_3d {
            if args.model_3d || args.full {
                // Default to project-relative paths (KIPRJMOD) for easier setup
                // Use --project-relative flag to force global paths if needed
                let model_path = if args.project_relative {
                    format!("${{KICAD6_3DMODEL_DIR}}/e2k.3dshapes/{}.wrl", sanitize_name(&model_info.title))
                } else {
                    format!("${{KIPRJMOD}}/e2k.3dshapes/{}.wrl", sanitize_name(&model_info.title))
                };

                ki_footprint.model_3d = Some(kicad::Ki3dModel {
                    path: model_path,
                    offset: (0.0, 0.0, 0.0),
                    scale: (1.0, 1.0, 1.0),
                    rotate: (0.0, 0.0, 0.0),
                });
            }
        }

        // Export footprint
        let exporter = FootprintExporter::new();
        let footprint_data = exporter.export(&ki_footprint)?;
        lib_manager.write_footprint(&ki_footprint.name, &footprint_data)?;

        println!("✓ Footprint converted: {}", ki_footprint.name);
    }

    // Process 3D model (if requested)
    if args.model_3d || args.full {
        if let Some(model_info) = &component_data.model_3d {
            log::info!("Converting 3D model...");

            // Try to download 3D models, but don't fail if they're not available
            match api.download_3d_obj(&model_info.uuid) {
                Ok(obj_data) => {
                    match api.download_3d_step(&model_info.uuid) {
                        Ok(step_data) => {
                            let exporter = ModelExporter::new();
                            let wrl_data = exporter.obj_to_wrl(&obj_data)?;
                            let step_data = exporter.export_step(&step_data)?;

                            let model_name = sanitize_name(&model_info.title);
                            lib_manager.write_3d_model(&model_name, &wrl_data, &step_data)?;

                            println!("✓ 3D model converted: {}", model_name);
                        }
                        Err(e) => {
                            log::warn!("Failed to download STEP model: {}", e);
                            println!("⚠ 3D model partially available (STEP download failed)");
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to download OBJ model: {}", e);
                    println!("⚠ 3D model not available (files not found on server)");
                }
            }
        } else {
            log::warn!("No 3D model metadata available for this component");
        }
    }

    println!("\n✓ Conversion complete!");
    println!("Output directory: {}", args.output.display());

    Ok(())
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
