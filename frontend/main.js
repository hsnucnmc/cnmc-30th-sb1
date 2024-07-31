let img = -1;
let image_name = "train_right.png";
const train_width = 350; // TODO: adjust with image size
const train_height = 263;

const main_canvas = document.getElementById("main-canvas");

function resizeCanvas() {
    main_canvas.width = window.innerWidth;
    main_canvas.height = window.innerHeight;
}    

window.addEventListener("resize", resizeCanvas);
resizeCanvas();

let derail_img = new Image();
derail_img.src = "derail.png";

let status = "nothing";
let movement_start = 0;
let run_time = 1000.0; //?ms

let tracklist = new Map();

function drawRotatedImg(ctx, rotation_center_x, rotation_center_y, rotation_degree, object_x, object_y, img) {
    ctx.save();
    ctx.translate(rotation_center_x, rotation_center_y);
    ctx.rotate((Math.PI / 180) * rotation_degree);
    ctx.translate(-rotation_center_x, -rotation_center_y);
    // ctx.fillRect(rotation_center_x - 25, rotation_center_y - 25, 50, 50);
    ctx.drawImage(img, object_x, object_y);
    ctx.restore();
}

function bezierDerivative(coords, t) {
    const n = (coords.length / 2) - 1; // Number of control points

    if (n === 1) {
        // Quadratic Bezier curve
        const [x0, y0, x1, y1, x2, y2] = coords;

        const invT = 1 - t;

        const dx = 2 * (1 - t) * (x1 - x0) + 2 * t * (x2 - x1);
        const dy = 2 * (1 - t) * (y1 - y0) + 2 * t * (y2 - y1);

        return { dx: dx, dy: dy };
    } else if (n === 2) {
        // Cubic Bezier curve
        const [x0, y0, x1, y1, x2, y2, x3, y3] = coords;

        const invT = 1 - t;
        const invT2 = invT * invT;
        const t2 = t * t;

        const dx = 3 * invT2 * (x1 - x0) + 6 * t * invT * (x2 - x1) + 3 * t2 * (x3 - x2);
        const dy = 3 * invT2 * (y1 - y0) + 6 * t * invT * (y2 - y1) + 3 * t2 * (y3 - y2);

        return { dx: dx, dy: dy };
    } else {
        throw new Error("Unsupported number of control points. Supported types are quadratic (3 points) and cubic (4 points).");
    }
}

function drawtrack(id, cordlist, color, thickness) {
    /**
     * @param id id
     * //@param level beziern (n is the level) 2-4 this should handle in this funciton 
     * @param cordlist a list of cord length 4 6 8
     * @param color color in hex
     * @param thickness just width
     */
    let main_canvas = document.getElementById("main-canvas");
    let main_context = main_canvas.getContext("2d");
    main_context.beginPath();
    main_context.lineWidth = thickness;
    main_context.moveTo(cordlist[0], cordlist[1]);
    main_context.strokeStyle = color;
    if (cordlist.length == 4) {
        main_context.bezierCurveTo(cordlist[0], cordlist[1], cordlist[2], cordlist[3], cordlist[2], cordlist[3]);
    } else if (cordlist.length == 6) {
        main_context.bezierCurveTo(cordlist[2], cordlist[3], cordlist[2], cordlist[3], cordlist[4], cordlist[5]);
    } else {
        main_context.bezierCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5], cordlist[6], cordlist[7]);
    }
    main_context.stroke();
    tracklist.set(id, cordlist);
    // TODO untest but should be good
}

function redraw(trainlist, st, duration) {
    /**
    * @param trainlist a list of param including (trainid, trackid)
    * @param st starting time
    * @param duration uh the duration?
    */

    let main_canvas = document.getElementById("main-canvas");
    let main_context = main_canvas.getContext("2d");

    if (movement_start == -1) {
        movement_start = time;
    }

    if (img == -1) {//? default image
        if (!image_name) image_name = "train_right.png";
        img = new Image(); // Create new img element
        img.src = image_name;
    }
    // TODO train behaviour different from the lebel of beizer curve
    // realtime = time - movement_start;
    // // let x_pos = (time - movement_start) / run_time * (main_canvas.clientWidth + train_width) - train_width;
    // // TODO without considering train width itself
    // let x_pos = ((time + run_time - realtime) / run_time) * dotlist[2] + (realtime / run_time) * dotlist[4];
    // let y_pos = ((time + run_time - realtime) / run_time) * dotlist[3] + (realtime / run_time) * dotlist[5];//kinda make some sense?
    // drawRotatedImg(main_context, x_pos + train_width / 2, y_pos + train_height / 2, 1 * Math.sin(2 * Math.PI * 30 * (time - movement_start) / run_time), x_pos, y_pos, img);

    // if (x_pos > main_canvas.clientWidth) {
    //     status = "nothing";
    //     movement_start = time;
    // }
    for (i = 0; i < trainlist.length; i++) {
        trainid = trainlist[i].trainid;
        trackid = trainlist[i].trackid;
        cordlist = tracklist.get(trackid);
        realt = (time - movement_start) / duration;
        if (cordlist.length == 4) {
            // 2point curve
            var x_pos = (1 - realt) * cordlist[0] + realt * cordlist[2];
            var y_pos = (1 - realt) * cordlist[1] + realt * cordlist[3];
        } else if (cordlist.length == 6) {
            // one cp
            var x_pos = (1 - realt) * (1 - realt) * cordlist[0] + 2 * (1 - realt) * realt * cordlist[2] + realt * realt * cordlist[4];
            var y_pos = (1 - realt) * (1 - realt) * cordlist[1] + 2 * (1 - realt) * realt * cordlist[3] + realt * realt * cordlist[5];
        } else if (cordlist.length == 8) {
            // standard 2 cp
            var x_pos = (1 - realt) * (1 - realt) * (1 - realt) * cordlist[0] + 3 * (1 - realt) * (1 - realt) * realt * cordlist[2] + 3 * (1 - realt) * realt * realt * cordlist[4] + realt * realt * realt * cordlist[6];
            var y_pos = (1 - realt) * (1 - realt) * (1 - realt) * cordlist[1] + 3 * (1 - realt) * (1 - realt) * realt * cordlist[3] + 3 * (1 - realt) * realt * realt * cordlist[5] + realt * realt * realt * cordlist[7];
        }
        //! not handling out of bound problem
        // now detrive
        dresult = bezierDerivative(cordlist, realt);
        slope = dresult.dy / dresult.dx;
        deg = Math.atanh(slope); //! not sure if atanh can work
        drawRotatedImg(main_context, x_pos, y_pos, deg, x_pos, y_pos, img);
    }

    window.requestAnimationFrame(redraw);
}

window.requestAnimationFrame(redraw);

let url = new URL(window.location.href);
console.log((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + url.pathname + "ws");
let socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + url.pathname + "ws");

socket.onopen = (event) => {
    // socket.send("position\n" + left_bound + " " + right_bound);
    socket.onmessage = (msg) => {
        console.log(msg);
        let msg_split = msg.data.split("\n");
        // * ! BLIND start here
        let row_count = 1;
        //prase here
        switch (msg_split[0]) {
            case "train":
                // code block
                args = msg_split[1].split(" ");
                // train_id=args[0];
                train_id = 0;//currently only support one train
                track_id = args[1];
                start_t = args[2];
                duration = args[3];
                img = new Image();
                img.src = msg_split[2];
                break;
            case "track":
                // console.log("not yet implment");
                trackcount = msg_split[1];
                for (i = 2; i < msg_split.length; i++) {
                    args = msg_split[i].split(" ");
                    dotlist = args[1].split(";");
                    dotlist.shift();//remove first item
                    drawtrack(args[0], dotlist, args[2], args[3]);//lgtm
                }
                break;
        }
        console.log(run_time);
    };
    socket.onclose = (msg) => {
        main_canvas.hidden = true;
        document.getElementById("main-canvas").parentElement.append(derail_img);
    };
};

// update click on demand
window.addEventListener("click", function (event) {
    mousePos = { x: event.clientX, y: event.clientY };
    //TODO finish this
    socket.send("click\n" + "0");
    //if mousePos is on anyone of the trains
    // mousePosText.textContent = `(${mousePos.x}, ${mousePos.y})`;
    //     click return which train have been clicked and its id one number
    // NOT IMPLMENT YET
});