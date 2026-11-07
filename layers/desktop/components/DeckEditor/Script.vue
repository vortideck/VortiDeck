<template lang="pug">
div#drop-button(class="rounded-lg flex flex-col gap-y-4 border-[1px] w-full h-full px-6 py-5 bg-white drop-shadow-sm")
    div#dictDefaultMessage(class="hidden")
        div(class="flex flex-col items-center justify-center gap-y-3 " )
            div(class="rounded-full p-[0.35rem] border-[1px] border-[#e5e5e5] bg-[#f6f6f6]")
                div(class="rounded-full p-2 border-[1px] border-gray-300 bg-white ")
                    AkarIconsCloudUpload(class="w-6 h-6 text-gray-500")
            div(class="flex flex-col gap-y-2")
                p(class="text-xl") Drop or Browse
                div(class="flex flex-col text-sm ")
                    p(class="text-gray-600  font-medium") Max 10 MB files are allowed
                    p(class="text-gray-600") accepted file types: .py <!-- .txt, .bat, .cmd, .sh, .ps1, -->
    form#dropzone(action="/target" class="w-full h-full dropzone flex justify-center gap-x-4 items-center  hover:cursor-pointer ")
        


//- .table.table-striped#previews
//-   #template.file-row
//-     //  This is used as the file preview template 
//-     div
//-       span.preview
//-         img(data-dz-thumbnail)
//-     div
//-       p.name(data-dz-name)
//-       strong.error.text-danger(data-dz-errormessage)
//-     div
//-       p.size(data-dz-size)
//-       .progress.progress-striped.active(role="progressbar", aria-valuemin="0", aria-valuemax="100", aria-valuenow="0")
//-         .progress-bar.progress-bar-success(style="width:0%;", data-dz-uploadprogress)
//-     div
//-       button.btn.btn-primary.start
//-         i.glyphicon.glyphicon-upload
//-         span Start
//-       button.btn.btn-warning.cancel(data-dz-remove)
//-         i.glyphicon.glyphicon-ban-circle
//-         span Cancel
//-       button.btn.btn-danger.delete(data-dz-remove)
//-         i.glyphicon.glyphicon-trash
//-         span Delete
</template>

<script setup lang="ts">
import AkarIconsCloudUpload from "~icons/akar-icons/cloud-upload?width=24px&height=24px";

import Dropzone from "dropzone";

onMounted(() => {
  // var previewNode = document.querySelector("#template");
  // previewNode.id = "";
  // var previewTemplate = previewNode.parentNode.innerHTML;
  // previewNode.parentNode.removeChild(previewNode);
  const myDropzone = new Dropzone("#dropzone", {
    clickable: "#drop-button",
    paramName: "file",
    maxFiles: 1,
    maxFilesize: 1,
    autoQueue: false,
    acceptedFiles: ".py", //.txt, .bat, .cmd, .sh, .ps1,
    dictDefaultMessage: document.getElementById("dictDefaultMessage")?.innerHTML,
    // previewTemplate: previewTemplate,
    // previewsContainer: "#previews", // Define the container to display the previews
  });
  myDropzone.on("addedfile", (file) => {
    console.log(`File added: ${file.name}`);
  });

  myDropzone.on("maxfilesexceeded", (file) => {
    myDropzone.removeAllFiles();
    myDropzone.addFile(file);
  });
});
</script>

<style lang="scss">
.dropzone.dz-started {
  & .dz-message {
    display: none !important;
  }

  & .dz-preview {
    background-color: white;
  }

  & .dz-progress,
  .dz-error-message,
  .dz-success-mark,
  .dz-error-mark {
    display: none !important;
  }

  & .dz-details {
    display: flex;
    flex-direction: column;
    justify-content: center;
    text-align: center;
    color: black;
  }

  & .dz-image {
    display: flex;
    flex-direction: column;
    justify-content: center;
    text-align: center;
    color: black;
  }
}

/* Here are the other styles for Dropzone. 
These will give it the same look as in the
picture. Which I think is better for non image files.*/

/* Haven't changed the checkbox that comes after the loading bar.*/
</style>
