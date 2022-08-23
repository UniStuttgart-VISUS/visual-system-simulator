export basedir="./assets"

for config in "astigmatism" "cataract-strong" "cataract-weak" "color-achromatopsia" "color-blue-blindness" "color-blue-weakness" "color-green-blindness" "color-green-weakness" "color-red-blindness" "color-red-weakness" "glaucoma-from-map" "glaucoma-medium" "glaucoma-strong" "glaucoma-weak" "hyperopia-minus2d" "kaputtness_overload" "macular-degeneration-medium" "macular-degeneration-weak" "macular-degeneration-strong" "myopia-plus2d" "nyctalopia" "normal" "nyctalopia-from-map" "presbyopia-600mm"
do
	for image in "cubes.rgbd.png" "test-colorblind.png" "test-calibration.png" "marketplace.png"
	do
		target/release/vss -c assets/configs/${config}.json $basedir/$image --output "{{dirname}}/config_tests/${config}/${image}.{{extension}}"
	done
done
