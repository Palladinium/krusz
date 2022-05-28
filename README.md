# KRUSZ

```
           ││││││││││
           ││││││││││
           ││││││││││
           ││││││││││
           ││││││││││
           ││││││││││
           ││││││││││
           ││││││││││
  ╔════════╧╧╧╧╧╧╧╧╧╧═════════╗
  ║                           ║
  VvVvVvVvVvVvVvVvVvVvVvVvVvVvV
                ♪
  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
```

A tiny utility to bitcrush sounds.

## Usage
    krusz [FLAGS] [OPTIONS] --input <input>

## Flags
    -h, --help       Prints help information
    -p, --play       Play the KRUSZED sound
    -V, --version    Prints version information

## Options
    -b, --bit-depth <bit-depth>            Target bit depth. Default: 16-bit depth
    -i, --input <input>                    The input file to KRUSZ
        --interpolation <interpolation>    Interpolation method for resampling. Available: Nearest, Linear. Default: Nearest
    -o, --output <output>                  The output KRUSZED file. Supported formats: WAV
    -s, --sample-rate <sample-rate>        Target sample rate. Default: 44100 Hz

