import("./pkg").then(m => {
    let sim = m.Simulator.create_and_run("vss-container");

    let image = new Image();
    image.onload = () => {
        let canvas = document.createElement("canvas");
        let context = canvas.getContext("2d");
        context.width = image.width;
        context.height = image.height;
        context.drawImage(image, 0, 0);
        let buffer = new Uint8Array(context.getImageData(0, 0, image.width, image.height).data.buffer);
        console.log("Posting frame", image.width, image.height, buffer);
        sim.post_frame(buffer, image.width, image.height);
    };
    image.onerror = () => {
        console.warn("Cannot load frame image", image.src);
    };
    image.src = "marketplace.png"

    let imageUpload = document.getElementById("imageUpload");
    imageUpload.addEventListener("change", (event) => {
        const file = event.target.files[0];
        if (file) {
            const reader = new FileReader();
            reader.addEventListener("load", (event) => {
                image.src = event.target.result;
            });
            reader.readAsDataURL(file);
        }
    });
});
