package main

import (
	"fmt"
	"image"
	"os"
	"path/filepath"
	"strings"

	"github.com/fogleman/gg"
	"github.com/solovev/gopsd"
)

func main() {
	files, err := filepath.Glob("*.psd")
	checkError(err)

	for _, f := range files {
		fmt.Println("Processing " + f + " ...\n")
		fname := strings.TrimSuffix(f, filepath.Ext(f))
		doc, err := gopsd.ParseFromPath("./" + f)
		checkError(err)
		dirname := "./sliced-images-" + fname
		os.Mkdir(dirname, 0777)

		for _, layer := range doc.Layers {
			fmt.Println(layer.ToString())
			extension := strings.ToLower(filepath.Ext(layer.Name))

			if extension == ".png" || extension == ".jpg" {
				extractLayer(layer, dirname, extension, doc)
			} else {
				fmt.Println("No supported extensions specified (png/jpg), skipping...")
			}
		}
	}
}

func extractLayer(layer *gopsd.Layer, dirname string, filetype string, doc *gopsd.Document) {
	out, err := os.Create("./" + dirname + "/" + layer.Name)
	checkError(err)

	img, err := layer.GetImage()
	var res gg.Context
	checkError(err)

	// if the string starts with _, then we keep the original PSD border size (no trimming)
	if strings.HasPrefix(layer.Name, "_") {
		fmt.Println("Leaving original size...")
		res = processLayer(img, layer, int(doc.Width), int(doc.Height), false)
	} else {
		fmt.Println("Cropping layer...")
		res = processLayer(img, layer, int(layer.Rectangle.Width), int(layer.Rectangle.Height), true)
	}

	if filetype == ".png" {
		res.SavePNG(out.Name())
	} else {
		gg.SaveJPG(out.Name(), res.Image(), 100)
	}
}

func processLayer(img image.Image, layer *gopsd.Layer, width int, height int, cropped bool) gg.Context {
	fmt.Println("Cropping layer...")
	dc := gg.NewContext(width, height)
	var x, y int

	// If we crop the layer, image should be drawn at 0,0
	if cropped {
		x = 0
		y = 0
	} else {
		x = int(layer.Rectangle.X)
		y = int(layer.Rectangle.Y)
	}

	dc.DrawImage(img, x, y)
	return *dc
}

func checkError(err error) {
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
