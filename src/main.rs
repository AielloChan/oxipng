#![warn(trivial_casts, trivial_numeric_casts, unused_import_braces)]
#![deny(missing_debug_implementations, missing_copy_implementations)]

use clap::{App, AppSettings, Arg, ArgMatches};
use oxipng::AlphaOptim;
use oxipng::Deflaters;
use oxipng::Headers;
use oxipng::Options;
use oxipng::PngResult;
use oxipng::{InFile, OutFile};
use std::collections::HashSet;
use std::fs::DirBuilder;
use std::path::PathBuf;
use std::process::exit;

fn main() {
    let matches = App::new("oxipng")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Joshua Holmer <jholmer.in@gmail.com>")
        .about("Losslessly improves compression of PNG files")
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::UnifiedHelpMessage)
        .arg(Arg::with_name("files")
            .help("File(s) to compress (use \"-\" for stdin)")
            .index(1)
            .multiple(true)
            .use_delimiter(false)
            .required(true))
        .arg(Arg::with_name("optimization")
            .help("Optimization level - Default: 2")
            .short("o")
            .long("opt")
            .takes_value(true)
            .value_name("level")
            .possible_value("0")
            .possible_value("1")
            .possible_value("2")
            .possible_value("3")
            .possible_value("4")
            .possible_value("5")
            .possible_value("6"))
        .arg(Arg::with_name("backup")
            .help("Back up modified files")
            .short("b")
            .long("backup"))
        .arg(Arg::with_name("recursive")
            .help("Recurse into subdirectories")
            .short("r")
            .long("recursive"))
        .arg(Arg::with_name("output_dir")
            .help("Write output file(s) to <directory>")
            .long("dir")
            .takes_value(true)
            .value_name("directory")
            .conflicts_with("output_file")
            .conflicts_with("stdout"))
        .arg(Arg::with_name("output_file")
            .help("Write output file to <file>")
            .long("out")
            .takes_value(true)
            .value_name("file")
            .conflicts_with("output_dir")
            .conflicts_with("stdout"))
        .arg(Arg::with_name("stdout")
            .help("Write output to stdout")
            .long("stdout")
            .conflicts_with("output_dir")
            .conflicts_with("output_file"))
        .arg(Arg::with_name("preserve")
            .help("Preserve file attributes if possible")
            .short("p")
            .long("preserve"))
        .arg(Arg::with_name("pretend")
            .help("Do not write any files, only calculate compression gains")
            .short("P")
            .long("pretend"))
        .arg(Arg::with_name("strip-safe")
            .help("Strip safely-removable metadata objects")
            .short("s")
            .conflicts_with("strip"))
        .arg(Arg::with_name("strip")
            .help("Strip metadata objects ['safe', 'all', or comma-separated list]")
            .long("strip")
            .takes_value(true)
            .value_name("mode")
            .conflicts_with("strip-safe"))
        .arg(Arg::with_name("keep")
            .help("Strip all optional metadata except objects in the comma-separated list")
            .long("keep")
            .takes_value(true)
            .value_name("list")
            .conflicts_with("strip")
            .conflicts_with("strip-safe"))
        .arg(Arg::with_name("alpha")
            .help("Perform additional alpha optimizations")
            .short("a")
            .long("alpha"))
        .arg(Arg::with_name("interlace")
            .help("PNG interlace type")
            .short("i")
            .long("interlace")
            .takes_value(true)
            .value_name("0/1")
            .possible_value("0")
            .possible_value("1"))
        .arg(Arg::with_name("verbose")
            .help("Run in verbose mode")
            .short("v")
            .long("verbose")
            .conflicts_with("quiet"))
        .arg(Arg::with_name("quiet")
            .help("Run in quiet mode")
            .short("q")
            .long("quiet")
            .conflicts_with("verbose"))
        .arg(Arg::with_name("filters")
            .help("PNG delta filters (0-5) - Default: 0,5")
            .short("f")
            .long("filters")
            .takes_value(true)
            .validator(|x| {
                match parse_numeric_range_opts(&x, 0, 5) {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Invalid option for filters".to_owned()),
                }
            }))
        .arg(Arg::with_name("compression")
            .help("zlib compression levels (1-9) - Default: 9")
            .long("zc")
            .takes_value(true)
            .value_name("levels")
            .validator(|x| {
                match parse_numeric_range_opts(&x, 1, 9) {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Invalid option for compression".to_owned()),
                }
            }))
        .arg(Arg::with_name("strategies")
            .help("zlib compression strategies (0-3) - Default: 0-3")
            .long("zs")
            .takes_value(true)
            .validator(|x| {
                match parse_numeric_range_opts(&x, 0, 3) {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Invalid option for strategies".to_owned()),
                }
            }))
        .arg(Arg::with_name("window")
            .help("zlib window size - Default: 32k")
            .long("zw")
            .takes_value(true)
            .value_name("size")
            .possible_value("256")
            .possible_value("512")
            .possible_value("1k")
            .possible_value("2k")
            .possible_value("4k")
            .possible_value("8k")
            .possible_value("16k")
            .possible_value("32k"))
        .arg(Arg::with_name("no-bit-reduction")
            .help("No bit depth reduction")
            .long("nb"))
        .arg(Arg::with_name("no-color-reduction")
            .help("No color type reduction")
            .long("nc"))
        .arg(Arg::with_name("no-palette-reduction")
            .help("No palette reduction")
            .long("np"))
        .arg(Arg::with_name("no-reductions")
            .help("No reductions")
            .long("nx"))
        .arg(Arg::with_name("no-recoding")
            .help("No IDAT recoding unless necessary")
            .long("nz"))
        .arg(Arg::with_name("fix")
            .help("Enable error recovery")
            .long("fix"))
        .arg(Arg::with_name("zopfli")
            .help("Use the slower but better compressing Zopfli algorithm, overrides zlib-specific options")
            .short("Z")
            .long("zopfli"))
        .arg(Arg::with_name("threads")
            .help("Set number of threads to use - default 1.5x CPU cores")
            .long("threads")
            .short("t")
            .takes_value(true)
            .value_name("num")
            .validator(|x| {
                match x.parse::<usize>() {
                    Ok(val) => {
                        if val > 0 {
                            Ok(())
                        } else {
                            Err("Thread count must be >= 1".to_owned())
                        }
                    }
                    Err(_) => Err("Thread count must be >= 1".to_owned()),
                }
            }))
        .after_help("Optimization levels:
    -o 0  =>  --zc 3 --nz                  (0 or 1 trials)
    -o 1  =>  --zc 9                       (1 trial, determined heuristically)
    -o 2  =>  --zc 9 --zs 0-3 -f 0,5       (8 trials)
    -o 3  =>  --zc 9 --zs 0-3 -f 0-5       (24 trials)
    -o 4  =>  --zc 9 --zs 0-3 -f 0-5 -a    (24 trials + 6 alpha trials)
    -o 5  =>  --zc 3-9 --zs 0-3 -f 0-5 -a  (96 trials + 6 alpha trials)
    -o 6  =>  --zc 1-9 --zs 0-3 -f 0-5 -a  (180 trials + 6 alpha trials)

    Manually specifying a compression option (zc, zs, etc.) will override the optimization preset,
    regardless of the order you write the arguments.")
        .get_matches_from(wild::args());

    let (out_file, out_dir, opts) = match parse_opts_into_struct(&matches) {
        Ok(x) => x,
        Err(x) => {
            eprintln!("{}", x);
            exit(1)
        }
    };

    let files = collect_files(
        matches
            .values_of("files")
            .unwrap()
            .map(PathBuf::from)
            .collect(),
        &out_dir,
        &out_file,
        opts.recursive,
        true,
    );

    let res: PngResult<()> = files
        .into_iter()
        .map(|(input, output)| oxipng::optimize(&input, &output, &opts))
        .collect();

    if let Err(e) = res {
        eprintln!("{}", e);
        exit(1);
    }
}

fn collect_files(
    files: Vec<PathBuf>,
    out_dir: &Option<PathBuf>,
    out_file: &OutFile,
    recursive: bool,
    allow_stdin: bool,
) -> Vec<(InFile, OutFile)> {
    let mut in_out_pairs = Vec::new();
    let allow_stdin = allow_stdin && files.len() == 1;
    for input in files {
        let using_stdin = allow_stdin && input.to_str().map_or(false, |p| p == "-");
        if !using_stdin && input.is_dir() {
            if recursive {
                let files = input
                    .read_dir()
                    .unwrap()
                    .map(|x| x.unwrap().path().to_owned())
                    .collect();
                in_out_pairs.extend(collect_files(files, out_dir, out_file, recursive, false));
            } else {
                eprintln!("{} is a directory, skipping", input.display());
            }
            continue;
        };
        let out_file = if let Some(ref out_dir) = *out_dir {
            let out_path = Some(out_dir.join(input.file_name().unwrap()));
            OutFile::Path(out_path)
        } else {
            (*out_file).clone()
        };
        let in_file = if using_stdin {
            InFile::StdIn
        } else {
            InFile::Path(input)
        };
        in_out_pairs.push((in_file, out_file));
    }
    in_out_pairs
}

fn parse_opts_into_struct(
    matches: &ArgMatches,
) -> Result<(OutFile, Option<PathBuf>, Options), String> {
    let mut opts = if let Some(x) = matches.value_of("optimization") {
        if let Ok(opt) = x.parse::<u8>() {
            Options::from_preset(opt)
        } else {
            unreachable!()
        }
    } else {
        Options::default()
    };

    if let Some(x) = matches.value_of("interlace") {
        opts.interlace = x.parse::<u8>().ok();
    }

    if let Some(x) = matches.value_of("filters") {
        opts.filter = parse_numeric_range_opts(x, 0, 5).unwrap();
    }

    if let Some(x) = matches.value_of("compression") {
        opts.compression = parse_numeric_range_opts(x, 1, 9).unwrap();
    }

    if let Some(x) = matches.value_of("strategies") {
        opts.strategies = parse_numeric_range_opts(x, 0, 3).unwrap();
    }

    match matches.value_of("window") {
        Some("256") => opts.window = 8,
        Some("512") => opts.window = 9,
        Some("1k") => opts.window = 10,
        Some("2k") => opts.window = 11,
        Some("4k") => opts.window = 12,
        Some("8k") => opts.window = 13,
        Some("16k") => opts.window = 14,
        // 32k is default
        _ => (),
    }

    let out_dir = if let Some(x) = matches.value_of("output_dir") {
        let path = PathBuf::from(x);
        if !path.exists() {
            match DirBuilder::new().recursive(true).create(&path) {
                Ok(_) => (),
                Err(x) => return Err(format!("Could not create output directory {}", x)),
            };
        } else if !path.is_dir() {
            return Err(format!(
                "{} is an existing file (not a directory), cannot create directory",
                x
            ));
        }
        Some(path)
    } else {
        None
    };

    let out_file = if matches.is_present("stdout") {
        OutFile::StdOut
    } else if let Some(x) = matches.value_of("output_file") {
        OutFile::Path(Some(PathBuf::from(x)))
    } else {
        OutFile::Path(None)
    };

    if matches.is_present("alpha") {
        opts.alphas.insert(AlphaOptim::White);
        opts.alphas.insert(AlphaOptim::Up);
        opts.alphas.insert(AlphaOptim::Down);
        opts.alphas.insert(AlphaOptim::Left);
        opts.alphas.insert(AlphaOptim::Right);
    }

    if matches.is_present("backup") {
        opts.backup = true;
    }

    if matches.is_present("force") {
        opts.force = true;
    }

    if matches.is_present("recursive") {
        opts.recursive = true;
    }

    if matches.is_present("fix") {
        opts.fix_errors = true;
    }

    if matches.is_present("clobber") {
        opts.clobber = false;
    }

    if matches.is_present("pretend") {
        opts.pretend = true;
    }

    if matches.is_present("preserve") {
        opts.preserve_attrs = true;
    }

    if matches.is_present("quiet") {
        opts.verbosity = None;
    }

    if matches.is_present("verbose") {
        opts.verbosity = Some(1);
    }

    if matches.is_present("no-bit-reduction") {
        opts.bit_depth_reduction = false;
    }

    if matches.is_present("no-color-reduction") {
        opts.color_type_reduction = false;
    }

    if matches.is_present("no-palette-reduction") {
        opts.palette_reduction = false;
    }

    if matches.is_present("no-reductions") {
        opts.bit_depth_reduction = false;
        opts.color_type_reduction = false;
        opts.palette_reduction = false;
    }

    if matches.is_present("no-recoding") {
        opts.idat_recoding = false;
    }

    if let Some(hdrs) = matches.value_of("keep") {
        opts.strip = Headers::Keep(hdrs.split(',').map(|x| x.trim().to_owned()).collect())
    }

    if let Some(hdrs) = matches.value_of("strip") {
        let hdrs = hdrs
            .split(',')
            .map(|x| x.trim().to_owned())
            .collect::<Vec<String>>();
        if hdrs.contains(&"safe".to_owned()) || hdrs.contains(&"all".to_owned()) {
            if hdrs.len() > 1 {
                return Err(
                    "'safe' or 'all' presets for --strip should be used by themselves".to_owned(),
                );
            }
            if hdrs[0] == "safe" {
                opts.strip = Headers::Safe;
            } else {
                opts.strip = Headers::All;
            }
        } else {
            const FORBIDDEN_CHUNKS: [[u8; 4]; 5] =
                [*b"IHDR", *b"IDAT", *b"tRNS", *b"PLTE", *b"IEND"];
            for i in &hdrs {
                if FORBIDDEN_CHUNKS.iter().any(|chunk| chunk == i.as_bytes()) {
                    return Err(format!("{} chunk is not allowed to be stripped", i));
                }
            }
            opts.strip = Headers::Strip(hdrs);
        }
    }

    if matches.is_present("strip-safe") {
        opts.strip = Headers::Safe;
    }

    if matches.is_present("zopfli") {
        opts.deflate = Deflaters::Zopfli;
    }

    if let Some(x) = matches.value_of("threads") {
        opts.threads = x.parse::<usize>().unwrap();
    }

    Ok((out_file, out_dir, opts))
}

fn parse_numeric_range_opts(
    input: &str,
    min_value: u8,
    max_value: u8,
) -> Result<HashSet<u8>, String> {
    const ERROR_MESSAGE: &str = "Not a valid input";
    let mut items = HashSet::new();

    // one value
    if let Ok(one_value) = input.parse::<u8>() {
        if (min_value <= one_value) && (one_value <= max_value) {
            items.insert(one_value);
            return Ok(items);
        }
    }

    // a range ("A-B")
    let range_values = input.split('-').collect::<Vec<&str>>();
    if range_values.len() == 2 {
        let first_opt = range_values[0].parse::<u8>();
        let second_opt = range_values[1].parse::<u8>();
        if let (Ok(first), Ok(second)) = (first_opt, second_opt) {
            if min_value <= first && first < second && second <= max_value {
                for i in first..=second {
                    items.insert(i);
                }
                return Ok(items);
            }
        }
        return Err(ERROR_MESSAGE.to_owned());
    }

    // a list ("A,B[,…]")
    let list_items = input.split(',').collect::<Vec<&str>>();
    if list_items.len() > 1 {
        for value in list_items {
            if let Ok(value_int) = value.parse::<u8>() {
                if (min_value <= value_int)
                    && (value_int <= max_value)
                    && !items.contains(&value_int)
                {
                    items.insert(value_int);
                    continue;
                }
            }
            return Err(ERROR_MESSAGE.to_owned());
        }
        return Ok(items);
    }

    Err(ERROR_MESSAGE.to_owned())
}
