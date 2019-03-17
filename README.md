# GoSlicer

GoSlicer is a tiny utility tool that can be used to quickly extract layers from Photoshop PSD files.
This software takes heavy inspiration from [Slicy](https://www.macupdate.com/app/mac/42535/slicy), which unfortunately is not developed or maintained anymore.

### Prerequisites

The tool uses gopsd and gg to extract layers and manipulate the output.
Install them with:

```
go get github.com/fogleman/gg
go get github.com/solovev/gopsd
```

### Use

The tool will run on all psd files found in the current folder, and layers will be extracted according to their naming.
Given this example:

![screenshot-layers](https://raw.githubusercontent.com/fulviuus/goslicer/master/_assets/screenshot-layers.png | width=500)

Ths following steps will happen:

- A directory "sliced-images-[filename]" will be created, which will contain the extracted layers
- The output format will be selected according to the specified file extension - only jpg and png are currently supported. In the example, all layers will be extracted in png format, apart from the purple.jpg layer which will be extracted in jpg.
- Al layers are cropped by default to the size of the contained graphics. To avoid this, place an underscore before the layer name. In the example, the \_red-box.png layer won't be cropped and will keep the original image size.

## Libraries

- [gg](https://github.com/fogleman/gg) - Go Graphics - 2D rendering in Go with a simple API.
- [gopsd](https://github.com/solovev/gopsd) - Photoshop document parser in Golang

## Authors

- **Fulvio Venturelli** - _Initial work. Probably also last._

## License

This project is licensed under the [WTFPL](https://en.wikipedia.org/wiki/WTFPL) License.
