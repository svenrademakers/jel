<!doctype html>
<html lang="en">

<head>
    <!-- Required meta tags -->
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">

    <!-- Bootstrap CSS -->
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap@4.3.1/dist/css/bootstrap.min.css"
        integrity="sha384-ggOyR0iXCbMQv3Xipma34MD+dH/1fQ784/j6cY/iJTQUOhcWr7x9JvoRxT2MZw1T" crossorigin="anonymous">

    <title>Sven's Streaming</title>
</head>
<body>

    <!-- Optional JavaScript -->
    <link href="https://vjs.zencdn.net/7.18.1/video-js.css" rel="stylesheet" />

    <!-- jQuery first, then Popper.js, then Bootstrap JS -->
    <script src="https://code.jquery.com/jquery-3.3.1.slim.min.js"
        integrity="sha384-q8i/X+965DzO0rT7abK41JStQIAqVgRVzpbzo5smXKp4YfRvH+8abtTE1Pi6jizo"
        crossorigin="anonymous"></script>
    <script src="https://cdn.jsdelivr.net/npm/popper.js@1.14.7/dist/umd/popper.min.js"
        integrity="sha384-UO2eT0CpHqdSJQ6hJty5KVphtPhzWj9WO1clHTMGa3JDZwrnQq4sF86dIHNDz0W1"
        crossorigin="anonymous"></script>
    <script src="https://cdn.jsdelivr.net/npm/bootstrap@4.3.1/dist/js/bootstrap.min.js"
        integrity="sha384-JjSmVgyd0p3pXB1rRibZUAYoIIy6OrQ6VrjIEaFf/nJGzIxFDsf4x0xIM+B07jRM"
        crossorigin="anonymous"></script>
    <style>

    </style>
    <nav class="navbar navbar-expand-md navbar-dark bg-dark">
        <a class="navbar-brand" href="#"> <img class="d-inline-block align-top" src="rosengard_logo.png" width="30"
                height="30" alt="">Ronaldo Streaming
            </a>
        <button class="navbar-toggler" type="button" data-toggle="collapse" data-target="#navbarCollapse"
            aria-controls="navbarCollapse" aria-expanded="false" aria-label="Toggle navigation">
            <span class="navbar-toggler-icon"></span>
        </button>
        <div class="collapse navbar-collapse" id="navbarCollapse">
            <ul class="navbar-nav mr-auto">
                <!-- <li class="nav-item active">
                    <a class="nav-link" href="#">Home <span class="sr-only">(current)</span></a>
                </li> -->
                <!-- <li class="nav-item">
              <a class="nav-link" href="#">Link</a>
            </li>
            <li class="nav-item">
              <a class="nav-link disabled" href="#" tabindex="-1" aria-disabled="true">Disabled</a>
            </li> -->
            </ul>
        </div>
    </nav>
    <div class="container pt-4">
        <h4>Vittsjo - Rosengard <span class="badge badge-danger">Live</span></h4>
        <div class="row embed-responsive embed-responsive-16by9">
            <video-js id="my_video_1" class="vjs-default-skin embed-responsive-item" controls preload="auto" width="640"
                height="268">
                <source src="http://live.svenrademakers.com:8080/hls/.m3u8"
                    type="application/x-mpegURL">
            </video-js>

            <script src="node_modules/video.js/dist/video.min.js"></script>
            <script src="node_modules/@videojs/http-streaming/dist/videojs-http-streaming.min.js"></script>
            <script>
                var player = videojs('my_video_1');
            </script>
        </div>
        <div class="row pt-5">
            <h4> Schedule </h4>
            <table class="table table-responsive-md table-hover">
                <thead>
                    <tr>
                        <th scope="col">Date</th>
                        <th scope="col">Home</th>
                        <th scope="col">Away</th>
                        <th scope="col">Venue</th>
                        <th scope="col">Score</th>
                        <th scope="col">Watch</th>
                    </tr>
                </thead>
                <tbody>
                    <?php
                    $strJsonFileContents = file_get_contents("fixtures.json");
                    $array = json_decode($strJsonFileContents, true)["response"];
                    foreach ($array as &$fixture) {
                        echo "<tr>";
                        echo "<th scope=\"row\">" . date("F jS", $fixture["fixture"]["timestamp"]) . "</th>";
                        echo "<td>". $fixture["teams"]["home"]["name"]. "</td>";
                        echo "<td>". $fixture["teams"]["away"]["name"]. "</td>";
                        echo "<td>". $fixture["fixture"]["venue"]["name"] . "</td>";
                        echo "<td>". $fixture["goals"]["home"] . " - " . $fixture["goals"]["away"] . "</td>";
                        echo "</tr>";
                    }
                    ?> 
                </tbody>
            </table>
        </div>
    </div>
</body>

</html>