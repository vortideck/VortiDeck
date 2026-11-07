<template lang="pug">

div(class="w-full xl:w-[56.5rem] h-full flex flex-col items-start justify-start gap-y-2")
    h2(class="text-lg font-bold    ") Featured Posts
    //- div.article-horizontal(class="flex-col flex md:flex-row w-full  gap-x-[3rem] 3xl:gap-x-[3rem] h-auto text-[#51535b]")
    //-     nuxt-img(format="webp" provider='ipx' alt="wood floot cleaner header" class="rounded-xl w-full md:w-1/2 max-h-[20rem] xs:h-[12rem] sm:h-[16rem] lg:min-h-[18rem] object-cover" src="https://www.expertreviews.co.uk/sites/expertreviews/files/2022/04/best_wood_floor_cleaner_-_hero.jpg")
    //-     div(class="flex flex-col w-full gap-y-6  my-4")
    //-         div.article__info(class="flex w-full gap-x-2 items-center")
    //-             nuxt-img(format="webp" alt="netflix"  provider='ipx' class="rounded-full" src="https://images.ctfassets.net/4cd45et68cgf/Rx83JoRDMkYNlMC9MKzcB/2b14d5a59fc3937afd3f03191e19502d/Netflix-Symbol.png?w=700&h=456" width="40px" height="40px")
    //-             p Netflix
    //-             p .
    //-             p 12 minutes ago
    //-         .article__title(class="text-3xl 3xl:text-xl font-semibold  text-[#22252f]") 5 Reasons Why You Should Hire Professional Cleaning Services While Moving Out
    //-         .article__description(class="") Lorem ipsum dolor sit amet cons ectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip.
    //-         .article__footer(class="flex gap-x-2")
    //-             p(class="text-red-500 font-semibold") Movies
    //-             p .
    //-             p  5 min read
    
    div(class="grid grid-cols-1 sm:grid-cols-2 gap-[2rem] 2xl:gap-x-[4rem] gap-y-[5rem] w-full mt-12")
        div(class="bg-white w-full   overflow-hidden  flex flex-col items-start justify-start gap-y-2 " v-for="article in articles" data-aos="fade" data-aos-easing="ease-in-cubic" data-aos-offset="100"
        data-aos-duration="100" data-aos-delay="0" data-aos-once="true")
            NuxtLink(:to="`/blog/article/${article.slug}`" class="w-full h-auto")
                
                nuxt-img(:src="'https:' + article.content.image" provider="storyblok" format="webp" width="600" h="400" class="w-full rounded-xl drop-shadow-lg max-h-[12rem] xs:h-[12rem] sm:h-[16rem] lg:min-h-[18rem]  object-cover " )
            div(class="flex flex-col items-start justify-start  gap-y-2 h-full md:h-[9rem] w-full mt-1")
                h4(class="text-lg font-bold  ") {{ article.content.title }}
                div(class="rounded-xl w-auto bg-[#f8f6fc] text-[#20249b] px-3 py-[0.3rem] text-sm") {{ article.content.category_name }}
                p(class="line-clamp-3 ") {{ article.content.description }}
            div(class="flex gap-x-4 mt-4")
                nuxt-img(format="webp" provider="ipx" alt="author avatar"     class="rounded-full min-w-12 max-w-12  min-h-12 max-h-12 object-cover" :src="article.content.author[0].avatar" width="30px" height="30px")
                div(class="flex flex-col text-sm justify-center gap-y-1")
                    p(class="text-[#313337] font-bold") {{article.content.author[0].name}}
                    
                    p {{ article.published_at.split('T')[0] }} &#8226; 
                      LazyClientOnly
                        span {{ calcReadTime(renderRichText(article.content.content)) }}  min read
    ContentBreakerNearestServices.mt-6
    

        //- div.article-horizontal(class="flex-col flex md:flex-row w-full  gap-x-[3rem] 3xl:gap-x-[4rem] h-auto text-[#51535b]")
        //-     nuxt-img(format="webp" provider='ipx' alt="wood floot cleaner header" class="rounded-xl w-full md:w-2/5" src="https://www.expertreviews.co.uk/sites/expertreviews/files/2022/04/best_wood_floor_cleaner_-_hero.jpg")
        //-     div(class="flex flex-col w-full gap-y-6 text-2xl my-4")
        //-         div.article__info(class="flex w-full gap-x-2 items-center")
        //-             nuxt-img(format="webp" alt="netflix"  provider='ipx' class="rounded-full" src="https://images.ctfassets.net/4cd45et68cgf/Rx83JoRDMkYNlMC9MKzcB/2b14d5a59fc3937afd3f03191e19502d/Netflix-Symbol.png?w=700&h=456" width="40px" height="40px")
        //-             p Netflix
        //-             p .
        //-             p 12 minutes ago
        //-         .article__title(class="text-3xl 3xl:text-5xl font-semibold  text-[#22252f]") 5 Reasons Why You Should Hire Professional Cleaning Services While Moving Out
        //-         .article__description(class="") Lorem ipsum dolor sit amet cons ectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip.
        //-         .article__footer(class="flex gap-x-2")
        //-             p(class="text-red-500 font-semibold") Movies
        //-             p .
        //-             p  5 min read

        //- h2(class="gray-600 text-4xl md:text-7xl  text-start font-semibold text-[#22252f] w-full md:mt-9") Articles
        //- div.blog__articles(class="flex flex-col lg:flex-row w-full lg:justify-start h-auto items-start justify-center gap-y-12 gap-x-[2.5rem]   ") 
        //-     BlogItem(v-for="article in articles"   :key="article.uuid" :article="article" )
        //-         template(v-slot:image)
        //-             nuxt-img(:src="'https:' + article.content.image" :alt="article.content.title" provider="storyblok" format="webp" width="300" h="200" class="w-full h-full object-cover rounded-xl ")
        //- div.blog__latest(class="flex flex-col  w-full gap-y-12 mt-9")
        //-     .flex.justify-between.items-center.w-full
        //-         h2(class="gray-600 text-4xl md:text-7xl  text-start font-semibold text-[#22252f]") Latest News
        //-         button(class="text-red-600 text-lg md:text-2xl font-semibold") See all ->
        //-     .blog__latest__content(class="flex w-full gap-x-4")
        //-         .news(class="flex flex-col w-1/5 gap-y-6")
        //-             nuxt-img(class="aspect-square object-cover rounded-3xl" provider="ipx" src="https://www.eurocarparts.com/blog/ecp-wp-content/uploads/2016/10/upholstery.jpg" format="webp")
        //-             p.blog__latest__content__info(class="flex w-full gap-x-2 items-center")
        //-                 nuxt-img(format="webp" provider="ipx" alt="netlix logo"     class="rounded-full" src="https://images.ctfassets.net/4cd45et68cgf/Rx83JoRDMkYNlMC9MKzcB/2b14d5a59fc3937afd3f03191e19502d/Netflix-Symbol.png?w=700&h=456" width="30px" height="30px")
        //-                 p Netflix
        //-                 p .
        //-                 p 12 minutes ago
        //-             p.blog_latest__content__title(class="text-4xl font-semibold text-[#22252f]") tsast

</template>

<script setup lang="ts">
import PaintEmoji from "~icons/noto/paintbrush";
import BookEmoji from "~icons/noto/orange-book";
import CinoEmoji from "~icons/noto/cinema";
import BulbEmoji from "~icons/noto/light-bulb";

import MdiSearch from "~icons/mdi/search";

defineProps({
  articles: Array,
});
</script>

<style lang="scss"></style>
