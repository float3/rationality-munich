import fs from 'node:fs/promises';
import path from 'node:path';
import {events} from '../lib/content.ts';
const root=path.resolve(import.meta.dirname,'..');
// The 3.7 MB original is not in the repository; only the webp it produces is.
// Drop the full-size photograph next to it to regenerate (see README).
const photo=path.join(root,'public/images/munich.jpg');
if(await fs.stat(photo).then(()=>true,()=>false)){const {default:sharp}=await import('sharp');await sharp(photo).resize({width:1600,withoutEnlargement:true}).webp({quality:82}).toFile(path.join(root,'public/images/munich.webp'))}
const escapeText=value=>value.replaceAll('\\','\\\\').replaceAll('\n','\\n').replaceAll(';','\\;').replaceAll(',','\\,');
const compactDate=value=>value.replaceAll('-','').replaceAll(':','');
function fold(line){let result='',current='',bytes=0;for(const char of line){const size=Buffer.byteLength(char);if(bytes+size>74){result+=current+'\r\n ';current='';bytes=1}current+=char;bytes+=size}return result+current}
function calendar(items){const lines=['BEGIN:VCALENDAR','VERSION:2.0','PRODID:-//Rationality Munich Concept//Events//EN','CALSCALE:GREGORIAN','METHOD:PUBLISH','X-WR-CALNAME:Munich events — September 2026 preview','X-WR-TIMEZONE:Europe/Berlin'];for(const event of items){lines.push('BEGIN:VEVENT',`UID:concept-${event.id}@rationality-munich.local`,'DTSTAMP:20260922T120000Z',`DTSTART:${compactDate(event.start)}`);if(event.end)lines.push(`DTEND:${compactDate(event.end)}`);lines.push(`SUMMARY:${escapeText(event.title)}`,`LOCATION:${escapeText(event.place)}`,`DESCRIPTION:${escapeText(event.summary+'\nSaved 22 September 2026. Check the original announcement for changes: '+event.source)}`,`URL:${event.source}`,'END:VEVENT')}lines.push('END:VCALENDAR');return lines.map(fold).join('\r\n')+'\r\n'}
await fs.mkdir(path.join(root,'public/downloads'),{recursive:true});
for(const event of events)await fs.writeFile(path.join(root,`public/downloads/${event.id}.ics`),calendar([event]));
await fs.writeFile(path.join(root,'public/downloads/munich-events.ics'),calendar(events));
console.log('Prepared the calendar downloads.');
