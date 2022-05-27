var fixtures = [];
class Match {
    constructor(fixture_id) {
        this.fixture_id = fixture_id;
    }

    get home() {
        return fixtures[this.fixture_id]["home"];
    }

    get away() {
        return fixtures[this.fixture_id]["away"];
    }

    get venue() {
        return fixtures[this.fixture_id]["venue"];
    }

    get score() {
        return fixtures[this.fixture_id]["score"];
    }

    get date() {
        function get_date(date) {
            const nth = function (d) {
                const dString = String(d);
                const last = +dString.slice(-2);
                if (last > 3 && last < 21) return 'th';
                switch (last % 10) {
                    case 1:
                        return "st";
                    case 2:
                        return "nd";
                    case 3:
                        return "rd";
                    default:
                        return "th";
                }
            }
            let output = "<div>" + date.getMonth() + " " + date.getDay() + " " + nth(date.getDay()) + "</div>";
            output += "<div>" + date.getHours() + ":" + date.getMinutes() + "</div>";
            return output;
        }
        let date = new Date(0);
        date.setUTCSeconds(fixtures[this.fixture_id]["timestamp"]);
        return "<div>" + $.format.date(date, "D MMM") + "</div><div>" + $.format.date(date, "HH:mm");
    }
}

$(document).ready(function () {
    $.get("matches", function (data) {
        fixtures = data;
        let x = upcoming_fixture();
        set_current_fixture(x);
        load_schedule_table(x);
    });
    var player = videojs('my_video_1', {
        autoplay: true,
        liveui: true,
        inactivityTimeout: 0,
    }, function () {
        videojs.log("player loaded", this.currentSrc());
    });
})

function load_schedule_table(x) {
    const index = Object.keys(fixtures).findIndex(k => k === x);
    let output = "";
    for (let i = Math.max(0, index - 2); i < Object.entries(fixtures).length; i++) {
        const match = new Match(Object.keys(fixtures)[i]);
        // disable previous fixtures
        if (i < index) {
            output += "<tr class=\"disabled\">";
        } else {
            output += "<tr>";
        }
        output += "<th scope=\"row\">" + match.date + "</th>";
        output += "<td>" + match.home +
            "</td>";
        output += "<td>" + match.away +
            "</td>";
        output += "<td>" + match.venue +
            "</td>";
        output += "<td>" + match.score +
            "</td>";
        if (has_watch(match.fixture_id)) {
            output += "<td><button type=\"button\" onclick=\"set_current_fixture(new Match(" + match.fixture_id + "))\" class=\"btn btn-outline-primary\">Watch</button></td>";
        }
        output += "</tr>";
    }
    $("#schedule_table").html(output);
}

function upcoming_fixture() {
    for (const [key, value] of Object.entries(fixtures)) {
        let game_time = fixtures[key]["timestamp"];
        if (game_time * 1000 > Date.now()) {
            return key;
        }
    }
}

function has_watch(fixt_id) {
    // return file_exists("matches/hls/".fixt_id.
    //         ".m3u8") ||
    //     file_exists("matches/dash/".fixt_id.
    //         ".mpd");
    return false;
}

function set_current_fixture(match, live = false) {
    let fixture = new Match(match);
    $("#current_title").text(fixture.home + " - " + fixture.away);
    if (live) {
        $("#current_title").append(" <span class=\"badge badge-danger\">Live</span>");
    }

    var hls_source = $("<source>", {
        type: "application / x - mpegURL ",
        "src": "hls/" + fixture.fixture_id + ".m3u8",
    });

    var dash_source = $("<source>", {
        type: "application/dash+xml",
        "src": "dash/" + fixture.fixture_id + ".mpd",
    });

    $(".my_video_1").html(hls_source);
    $(".my_video_1").append(dash_source);
}