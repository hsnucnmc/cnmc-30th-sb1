function drawRotatedImg(ctx, rotation_center_x, rotation_center_y, rotation_degree, object_x, object_y, img) {
    ctx.save();
    ctx.translate(rotation_center_x, rotation_center_y);
    ctx.rotate((Math.PI / 180) * rotation_degree);
    ctx.translate(-rotation_center_x, -rotation_center_y);
    ctx.drawImage(img, object_x, object_y, train_width, train_height);
    ctx.restore();
}

function drawSingleTie(ctx, track, length, current_t) {
    let pos = bezierPoint(track.cordlist, current_t);
    let un_normalized = bezierDerivative(track.cordlist, current_t);
    let speed = Math.sqrt(Math.pow(un_normalized.dx, 2) + Math.pow(un_normalized.dy, 2));
    let dx = un_normalized.dx / speed * length / 2;
    let dy = un_normalized.dy / speed * length / 2;
    ctx.beginPath();
    ctx.moveTo(pos.x + dy, pos.y - dx);
    ctx.lineTo(pos.x - dy, pos.y + dx);
    ctx.stroke();
}

function drawTrack(ctx, track) {
    let cordlist = track.cordlist;
    let color = track.color;
    let thickness = track.thickness;

    ctx.beginPath();
    ctx.strokeStyle = color;
    ctx.lineWidth = thickness;
    ctx.moveTo(cordlist[0], cordlist[1]);
    if (cordlist.length == 4) {
        ctx.lineTo(cordlist[2], cordlist[3]);
    } else if (cordlist.length == 6) {
        ctx.quadraticCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5]);
    } else {
        ctx.bezierCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5], cordlist[6], cordlist[7]);
    }
    ctx.stroke();

    ctx.beginPath();
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = thickness * 0.75;
    ctx.moveTo(cordlist[0], cordlist[1]);
    if (cordlist.length == 4) {
        ctx.lineTo(cordlist[2], cordlist[3]);
    } else if (cordlist.length == 6) {
        ctx.quadraticCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5]);
    } else {
        ctx.bezierCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5], cordlist[6], cordlist[7]);
    }
    ctx.stroke();

    // TODO: adjust tie density base off of track length
    ctx.strokeStyle = color;
    ctx.lineWidth = thickness / 6;
    let n = 30.0 * track.length / 500.0;
    for (let i = 0.5 / n / 2; i < 1; i += 1 / n) {
        drawSingleTie(ctx, track, thickness * 2, i);
    }
    if (debugMode) {
        ctx.strokeStyle = "#000";
        drawSingleTie(ctx, track, thickness * 2, 0);
    }
}
