function loadImage(simulator, imageSrc) {
    let image = new Image();
    image.onload = () => {
        let canvas = document.createElement("canvas");
        let context = canvas.getContext("2d");
        context.width = image.width;
        context.height = image.height;
        context.drawImage(image, 0, 0);
        let imageData = context.getImageData(0, 0, image.width, image.height);
        let buffer = new Uint8Array(imageData.data.buffer);
        simulator.post_frame(buffer, image.width, image.height);
    };
    image.onerror = () => {
        console.warn("Cannot load frame image", image.src);
    };
    image.src = imageSrc;
}

function registerImageUpload(simulator) {
    let imageUpload = document.getElementById("imageUpload");
    imageUpload.addEventListener("change", (event) => {
        const file = event.target.files[0];
        if (file) {
            const reader = new FileReader();
            reader.addEventListener("load", (event) => {
                loadImage(simulator, event.target.result);
            });
            reader.readAsDataURL(file);
        }
    });
}

import("./pkg").then(m => {
    let simulator = m.Simulator.create_and_run("vss-container");
    loadImage(simulator, "marketplace.png");
    registerImageUpload(simulator);
});
