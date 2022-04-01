<?php
$curl = curl_init();

curl_setopt_array($curl, [
	CURLOPT_URL => "https://api-football-v1.p.rapidapi.com/v3/fixtures?season=2022&team=11075",
	CURLOPT_RETURNTRANSFER => true,
	CURLOPT_FOLLOWLOCATION => true,
	CURLOPT_ENCODING => "",
	CURLOPT_MAXREDIRS => 10,
	CURLOPT_TIMEOUT => 30,
	CURLOPT_HTTP_VERSION => CURL_HTTP_VERSION_1_1,
	CURLOPT_CUSTOMREQUEST => "GET",
	CURLOPT_HTTPHEADER => [
		"X-RapidAPI-Host: api-football-v1.p.rapidapi.com",
		"X-RapidAPI-Key: f777360069msh208ba92c3ad79cep1cf34fjsnd5c4ebf0cf7b"
	],
]);

$response = curl_exec($curl);
$err = curl_error($curl);

curl_close($curl);

if ($err) {
	echo "cURL Error #:" . $err;
} else {
	$myfile = fopen("fixtures.json", "w");
    fwrite($myfile, $response);
    fclose($myfile);
}